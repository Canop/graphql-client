use constants::*;
use failure;
use proc_macro2::{Ident, Span, TokenStream};
use query::QueryContext;
use selection::{Selection, SelectionFragmentSpread, SelectionItem};
use std::cell::Cell;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct GqlUnion {
    pub description: Option<String>,
    pub variants: BTreeSet<String>,
    pub is_required: Cell<bool>,
}

#[derive(Debug, Fail)]
#[fail(display = "UnionError")]
enum UnionError {
    #[fail(display = "Unknown type: {}", ty)]
    UnknownType { ty: String },
    #[fail(display = "Missing __typename in selection for {}", union_name)]
    MissingTypename { union_name: String },
}

type UnionVariantResult = Result<(Vec<TokenStream>, Vec<TokenStream>, Vec<String>), failure::Error>;

pub(crate) fn union_variants(
    selection: &Selection,
    query_context: &QueryContext,
    prefix: &str,
) -> UnionVariantResult {
    let mut children_definitions = Vec::new();
    let mut used_variants = Vec::with_capacity(selection.0.len());

    let variants: Result<Vec<TokenStream>, failure::Error> = selection
        .0
        .iter()
        // ignore __typename
        .filter(|item| {
            if let SelectionItem::Field(f) = item {
                f.name != TYPENAME_FIELD
            } else {
                true
            }
        })
        .map(|item| {
            let (on, fields) = match item {
                SelectionItem::Field(_) => Err(format_err!("field selection on union"))?,
                SelectionItem::FragmentSpread(SelectionFragmentSpread { fragment_name }) => {
                    let fragment = query_context
                        .fragments
                        .get(fragment_name)
                        .ok_or_else(|| format_err!("Unknown fragment: {}", &fragment_name))?;

                    (&fragment.on, &fragment.selection)
                }
                SelectionItem::InlineFragment(frag) => (&frag.on, &frag.fields),
            };
            let variant_name = Ident::new(&on, Span::call_site());
            used_variants.push(on.to_string());

            let new_prefix = format!("{}On{}", prefix, on);

            let variant_type = Ident::new(&new_prefix, Span::call_site());

            let field_object_type = query_context
                .schema
                .objects
                .get(on)
                .map(|_f| query_context.maybe_expand_field(&on, &fields, &new_prefix));
            let field_interface = query_context
                .schema
                .interfaces
                .get(on)
                .map(|_f| query_context.maybe_expand_field(&on, &fields, &new_prefix));
            let field_union_type = query_context
                .schema
                .unions
                .get(on)
                .map(|_f| query_context.maybe_expand_field(&on, &fields, &new_prefix));

            match field_object_type.or(field_interface).or(field_union_type) {
                Some(tokens) => children_definitions.push(tokens?),
                None => Err(UnionError::UnknownType { ty: on.to_string() })?,
            };

            Ok(quote! {
                #variant_name(#variant_type)
            })
        })
        .collect();

    let variants = variants?;

    Ok((variants, children_definitions, used_variants))
}

impl GqlUnion {
    pub(crate) fn response_for_selection(
        &self,
        query_context: &QueryContext,
        selection: &Selection,
        prefix: &str,
    ) -> Result<TokenStream, failure::Error> {
        let struct_name = Ident::new(prefix, Span::call_site());
        let derives = query_context.response_derives();

        // TODO: do this inside fragments
        let typename_field = selection.extract_typename(query_context);

        if typename_field.is_none() {
            Err(UnionError::MissingTypename {
                union_name: prefix.into(),
            })?;
        }

        let (mut variants, children_definitions, used_variants) =
            union_variants(selection, query_context, prefix)?;

        variants.extend(
            self.variants
                .iter()
                .filter(|v| used_variants.iter().find(|a| a == v).is_none())
                .map(|v| {
                    let v = Ident::new(v, Span::call_site());
                    quote!(#v)
                }),
        );

        Ok(quote! {
            #(#children_definitions)*

            #derives
            #[serde(tag = "__typename")]
            pub enum #struct_name {
                #(#variants),*
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deprecation::DeprecationStatus;
    use field_type::FieldType;
    use objects::{GqlObject, GqlObjectField};
    use selection::*;

    #[test]
    fn union_response_for_selection_complains_if_typename_is_missing() {
        let fields = vec![
            SelectionItem::InlineFragment(SelectionInlineFragment {
                on: "User".to_string(),
                fields: Selection(vec![SelectionItem::Field(SelectionField {
                    alias: None,
                    name: "firstName".to_string(),
                    fields: Selection(vec![]),
                })]),
            }),
            SelectionItem::InlineFragment(SelectionInlineFragment {
                on: "Organization".to_string(),
                fields: Selection(vec![SelectionItem::Field(SelectionField {
                    alias: None,
                    name: "title".to_string(),
                    fields: Selection(vec![]),
                })]),
            }),
        ];
        let mut context = QueryContext::new_empty();
        let selection = Selection(fields);
        let prefix = "Meow";
        let union = GqlUnion {
            description: None,
            variants: BTreeSet::new(),
            is_required: false.into(),
        };

        context.schema.objects.insert(
            "User".to_string(),
            GqlObject {
                description: None,
                name: "User".to_string(),
                fields: vec![
                    GqlObjectField {
                        description: None,
                        name: "firstName".to_string(),
                        type_: FieldType::Named("String".to_string()),
                        deprecation: DeprecationStatus::Current,
                    },
                    GqlObjectField {
                        description: None,
                        name: "lastName".to_string(),
                        type_: FieldType::Named("String".to_string()),

                        deprecation: DeprecationStatus::Current,
                    },
                    GqlObjectField {
                        description: None,
                        name: "createdAt".to_string(),
                        type_: FieldType::Named("Date".to_string()),
                        deprecation: DeprecationStatus::Current,
                    },
                ],
                is_required: false.into(),
            },
        );

        context.schema.objects.insert(
            "Organization".to_string(),
            GqlObject {
                description: None,
                name: "Organization".to_string(),
                fields: vec![
                    GqlObjectField {
                        description: None,
                        name: "title".to_string(),
                        type_: FieldType::Named("String".to_string()),
                        deprecation: DeprecationStatus::Current,
                    },
                    GqlObjectField {
                        description: None,
                        name: "created_at".to_string(),
                        type_: FieldType::Named("Date".to_string()),
                        deprecation: DeprecationStatus::Current,
                    },
                ],
                is_required: false.into(),
            },
        );

        let result = union.response_for_selection(&context, &selection, &prefix);

        assert!(result.is_err());

        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Missing __typename in selection for Meow"
        );
    }

    #[test]
    fn union_response_for_selection_works() {
        let fields = vec![
            SelectionItem::Field(SelectionField {
                alias: None,
                name: "__typename".to_string(),
                fields: Selection(vec![]),
            }),
            SelectionItem::InlineFragment(SelectionInlineFragment {
                on: "User".to_string(),
                fields: Selection(vec![SelectionItem::Field(SelectionField {
                    alias: None,
                    name: "firstName".to_string(),
                    fields: Selection(vec![]),
                })]),
            }),
            SelectionItem::InlineFragment(SelectionInlineFragment {
                on: "Organization".to_string(),
                fields: Selection(vec![SelectionItem::Field(SelectionField {
                    alias: None,
                    name: "title".to_string(),
                    fields: Selection(vec![]),
                })]),
            }),
        ];
        let mut context = QueryContext::new_empty();
        let selection = Selection(fields);
        let prefix = "Meow";
        let union = GqlUnion {
            description: None,
            variants: BTreeSet::new(),
            is_required: false.into(),
        };

        let result = union.response_for_selection(&context, &selection, &prefix);

        assert!(result.is_err());

        context.schema.objects.insert(
            "User".to_string(),
            GqlObject {
                description: None,
                name: "User".to_string(),
                fields: vec![
                    GqlObjectField {
                        description: None,
                        name: "__typename".to_string(),
                        type_: FieldType::Named(string_type()),
                        deprecation: DeprecationStatus::Current,
                    },
                    GqlObjectField {
                        description: None,
                        name: "firstName".to_string(),
                        type_: FieldType::Named(string_type()),
                        deprecation: DeprecationStatus::Current,
                    },
                    GqlObjectField {
                        description: None,
                        name: "lastName".to_string(),
                        type_: FieldType::Named(string_type()),
                        deprecation: DeprecationStatus::Current,
                    },
                    GqlObjectField {
                        description: None,
                        name: "createdAt".to_string(),
                        type_: FieldType::Named("Date".to_string()),
                        deprecation: DeprecationStatus::Current,
                    },
                ],
                is_required: false.into(),
            },
        );

        context.schema.objects.insert(
            "Organization".to_string(),
            GqlObject {
                description: None,
                name: "Organization".to_string(),
                fields: vec![
                    GqlObjectField {
                        description: None,
                        name: "__typename".to_string(),
                        type_: FieldType::Named(string_type()),
                        deprecation: DeprecationStatus::Current,
                    },
                    GqlObjectField {
                        description: None,
                        name: "title".to_string(),
                        type_: FieldType::Named("String".to_string()),
                        deprecation: DeprecationStatus::Current,
                    },
                    GqlObjectField {
                        description: None,
                        name: "createdAt".to_string(),
                        type_: FieldType::Named("Date".to_string()),
                        deprecation: DeprecationStatus::Current,
                    },
                ],
                is_required: false.into(),
            },
        );

        let result = union.response_for_selection(&context, &selection, &prefix);

        println!("{:?}", result);

        assert!(result.is_ok());

        assert_eq!(
            result.unwrap().to_string(),
            vec![
                "# [ derive ( Deserialize ) ] ",
                "pub struct MeowOnUser { # [ serde ( rename = \"firstName\" ) ] pub first_name : String , } ",
                "# [ derive ( Deserialize ) ] ",
                "pub struct MeowOnOrganization { pub title : String , } ",
                "# [ derive ( Deserialize ) ] ",
                "# [ serde ( tag = \"__typename\" ) ] ",
                "pub enum Meow { User ( MeowOnUser ) , Organization ( MeowOnOrganization ) }",
            ].into_iter()
                .collect::<String>(),
        );
    }
}

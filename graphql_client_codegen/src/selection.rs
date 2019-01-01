use constants::*;
use graphql_parser::query::SelectionSet;

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionField {
    pub alias: Option<String>,
    pub name: String,
    pub fields: Selection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionFragmentSpread {
    pub fragment_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionInlineFragment {
    pub on: String,
    pub fields: Selection,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectionItem {
    Field(SelectionField),
    FragmentSpread(SelectionFragmentSpread),
    InlineFragment(SelectionInlineFragment),
}

impl SelectionItem {
    pub fn as_typename(&self) -> Option<&SelectionField> {
        if let SelectionItem::Field(f) = self {
            if f.name == TYPENAME_FIELD {
                return Some(f);
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selection(pub Vec<SelectionItem>);

impl Selection {
    pub(crate) fn extract_typename(
        &self,
        context: &crate::query::QueryContext,
    ) -> Option<&SelectionField> {
        self.0.iter().filter_map(|f| f.as_typename()).next()
    }
}

impl<'a> ::std::convert::From<&'a SelectionSet> for Selection {
    fn from(selection_set: &SelectionSet) -> Selection {
        use graphql_parser::query::Selection;

        let mut items = Vec::new();

        for item in &selection_set.items {
            let converted = match item {
                Selection::Field(f) => SelectionItem::Field(SelectionField {
                    alias: f.alias.as_ref().map(|alias| alias.to_string()),
                    name: f.name.to_string(),
                    fields: (&f.selection_set).into(),
                }),
                Selection::FragmentSpread(spread) => {
                    SelectionItem::FragmentSpread(SelectionFragmentSpread {
                        fragment_name: spread.fragment_name.to_string(),
                    })
                }
                Selection::InlineFragment(inline) => {
                    SelectionItem::InlineFragment(SelectionInlineFragment {
                        on: inline
                            .type_condition
                            .clone()
                            .expect("missing \"on\" clause")
                            .to_string()
                            .replace("on ", ""),
                        fields: (&inline.selection_set).into(),
                    })
                }
            };
            items.push(converted);
        }

        Selection(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphql_parser;

    #[test]
    fn selection_extract_typename_simple_case() {
        let mut selection = Selection(Vec::new());
        let context = query::QueryContext::new_empty();

        assert!(selection.extract_typename(&context).is_none());
    }

    #[test]
    fn selection_from_graphql_parser_selection_set() {
        let query = r##"
        query {
          animal {
            isCat
            isHorse
            ...Timestamps
            barks
            ...on Dog {
                rating
            }
            pawsCount
            aliased: sillyName
          }
        }
        "##;
        let parsed = graphql_parser::parse_query(query).unwrap();
        let selection_set: &graphql_parser::query::SelectionSet = parsed
            .definitions
            .iter()
            .filter_map(|def| {
                if let graphql_parser::query::Definition::Operation(
                    graphql_parser::query::OperationDefinition::Query(q),
                ) = def
                {
                    Some(&q.selection_set)
                } else {
                    None
                }
            })
            .next()
            .unwrap();

        let selection: Selection = selection_set.into();

        assert_eq!(
            selection,
            Selection(vec![SelectionItem::Field(SelectionField {
                alias: None,
                name: "animal".to_string(),
                fields: Selection(vec![
                    SelectionItem::Field(SelectionField {
                        alias: None,
                        name: "isCat".to_string(),
                        fields: Selection(Vec::new()),
                    }),
                    SelectionItem::Field(SelectionField {
                        alias: None,
                        name: "isHorse".to_string(),
                        fields: Selection(Vec::new()),
                    }),
                    SelectionItem::FragmentSpread(SelectionFragmentSpread {
                        fragment_name: "Timestamps".to_string(),
                    }),
                    SelectionItem::Field(SelectionField {
                        alias: None,
                        name: "barks".to_string(),
                        fields: Selection(Vec::new()),
                    }),
                    SelectionItem::InlineFragment(SelectionInlineFragment {
                        on: "Dog".to_string(),
                        fields: Selection(vec![SelectionItem::Field(SelectionField {
                            alias: None,
                            name: "rating".to_string(),
                            fields: Selection(Vec::new()),
                        })]),
                    }),
                    SelectionItem::Field(SelectionField {
                        alias: None,
                        name: "pawsCount".to_string(),
                        fields: Selection(Vec::new()),
                    }),
                    SelectionItem::Field(SelectionField {
                        alias: Some("aliased".to_string()),
                        name: "sillyName".to_string(),
                        fields: Selection(Vec::new()),
                    }),
                ]),
            })])
        );
    }
}

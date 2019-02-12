use crate::deprecation::DeprecationStrategy;
use heck::SnakeCase;
use proc_macro2::{Ident, Span};
use std::path::{Path, PathBuf};
use syn::Visibility;

/// Used to configure code generation.
#[derive(Debug, Default)]
pub struct GraphQLClientCodegenOptions {
    /// Name of the operation we want to generate code for. If it does not match, we use all queries.
    pub operation_name: Option<String>,
    /// The name of implemention target struct.
    pub struct_name: Option<String>,
    /// The name of the module that will contains queries.
    pub module_name: Option<String>,
    /// Comma-separated list of additional traits we want to derive.
    additional_derives: Option<String>,
    /// The deprecation strategy to adopt.
    deprecation_strategy: Option<DeprecationStrategy>,
    /// Target module visibility.
    module_visibility: Option<Visibility>,
    /// A path to a file to include in the module to force Cargo to take into account changes in
    /// the query files when recompiling.
    query_file: Option<PathBuf>,
    /// A path to a file to include in the module to force Cargo to take into account changes in
    /// the schema files when recompiling.
    schema_file: Option<PathBuf>,
}

impl GraphQLClientCodegenOptions {
    /// Creates an empty options object with default params. It probably wants to be configured.
    pub fn new_default() -> GraphQLClientCodegenOptions {
        std::default::Default::default()
    }

    /// The module name, either one that was set explicitly, or the operation name, as snake case.
    pub(crate) fn module_name_ident(&self) -> Option<Ident> {
        self.module_name
            .as_ref()
            .or_else(|| self.operation_name.as_ref())
            .map(|s| s.to_snake_case())
            .map(|module_name| Ident::new(&module_name, Span::call_site()))
    }

    /// The visibility (public/private) to apply to the target module.
    pub(crate) fn module_visibility(&self) -> &Visibility {
        self.module_visibility
            .as_ref()
            .unwrap_or(&Visibility::Inherited)
    }

    /// The deprecation strategy to adopt.
    pub(crate) fn deprecation_strategy(&self) -> DeprecationStrategy {
        self.deprecation_strategy.clone().unwrap_or_default()
    }

    /// A path to a file to include in the module to force Cargo to take into account changes in
    /// the query files when recompiling.
    pub fn set_query_file(&mut self, path: PathBuf) {
        self.query_file = Some(path);
    }

    /// Comma-separated list of additional traits we want to derive.
    pub fn additional_derives(&self) -> Option<&str> {
        self.additional_derives.as_ref().map(String::as_str)
    }

    /// Comma-separated list of additional traits we want to derive.
    pub fn set_additional_derives(&mut self, additional_derives: String) {
        self.additional_derives = Some(additional_derives);
    }

    /// The deprecation strategy to adopt.
    pub fn set_deprecation_strategy(&mut self, deprecation_strategy: DeprecationStrategy) {
        self.deprecation_strategy = Some(deprecation_strategy);
    }

    /// The name of the module that will contains queries.
    pub fn set_module_name(&mut self, module_name: String) {
        self.module_name = Some(module_name);
    }

    /// Target module visibility.
    pub fn set_module_visibility(&mut self, visibility: Visibility) {
        self.module_visibility = Some(visibility);
    }

    /// The name of implemention target struct.
    pub fn set_struct_name(&mut self, struct_name: String) {
        self.struct_name = Some(struct_name);
    }

    /// Name of the operation we want to generate code for. If none is selected, it means all
    /// operations.
    pub fn set_operation_name(&mut self, operation_name: String) {
        self.operation_name = Some(operation_name);
    }

    /// A path to a file to include in the module to force Cargo to take into account changes in
    /// the schema files when recompiling.
    pub fn schema_file(&self) -> Option<&Path> {
        self.schema_file.as_ref().map(PathBuf::as_path)
    }

    /// A path to a file to include in the module to force Cargo to take into account changes in
    /// the query files when recompiling.
    pub fn query_file(&self) -> Option<&Path> {
        self.query_file.as_ref().map(PathBuf::as_path)
    }
}

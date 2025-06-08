use crate::configs::Context;
use crate::configs::Config;
use crate::error::BError;
use serde_json::Value;

pub struct WsProductData {
    name: String,
    project: String,
    product: String,
    arch: String,
    description: String,
}

impl Config for WsProductData {}

impl WsProductData {
    pub fn from_str(json_string: &str) -> Result<Self, BError> {
        let data: Value = Self::parse(json_string)?;
        Self::from_value(&data)
    }

    pub fn from_value(data: &Value) -> Result<Self, BError> {
        // Duplication from WsConfigData which is also keeping track of the name
        // for now leave it but should potentially move it
        let name: String = Self::get_str_value("name", &data, Some(String::from("NA")))?;
        let project: String = Self::get_str_value("project", &data, Some(name.clone()))?;
        let product: String = Self::get_str_value("product", &data, Some(name.clone()))?;
        let description: String =
            Self::get_str_value("description", &data, Some(String::from("NA")))?;
        let arch: String = Self::get_str_value("arch", &data, Some(String::from("NA")))?;

        Ok(WsProductData {
            name,
            project,
            product,
            arch,
            description,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn product(&self) -> &str {
        &self.product
    }

    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn arch(&self) -> &str {
        &self.arch
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn to_string(&self) -> String {
        let product_str: String = format!(
            "\"name\": \"{}\",\"arch\": \"{}\", \"description\": \"{}\"",
            self.name, self.arch, self.description
        );
        product_str.clone()
    }

    pub fn expand_ctx(&mut self, _ctx: &Context) -> Result<(), BError> {
        // Add call to expand ctx in any of the variables
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::data::WsProductData;

    #[test]
    fn test_ws_product_data_default() {
        let json_default_build_config = r#"
        {
            "version": "6"
        }"#;
        let data: WsProductData = WsProductData::from_str(json_default_build_config)
            .expect("Failed to parse product data");
        assert_eq!(data.name(), "NA");
        assert_eq!(data.project(), data.name());
        assert_eq!(data.product(), data.name());
        assert_eq!(data.description(), "NA");
        assert_eq!(data.arch(), "NA");
    }

    #[test]
    fn test_ws_product_data() {
        let json_default_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "description": "test description",
            "arch": "test-arch"
        }"#;
        let data: WsProductData = WsProductData::from_str(json_default_build_config)
            .expect("Failed to parse product data");
        assert_eq!(data.name(), "test-name");
        assert_eq!(data.project(), data.name());
        assert_eq!(data.product(), data.name());
        assert_eq!(data.description(), "test description");
        assert_eq!(data.arch(), "test-arch");
    }

    #[test]
    fn test_ws_project_name_data() {
        let json_default_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "project": "test-project",
            "description": "test description",
            "arch": "test-arch"
        }"#;
        let data: WsProductData = WsProductData::from_str(json_default_build_config)
            .expect("Failed to parse product data");
        assert_eq!(data.name(), "test-name");
        assert_eq!(data.project(), "test-project");
        assert_eq!(data.product(), data.name());
        assert_eq!(data.description(), "test description");
        assert_eq!(data.arch(), "test-arch");
    }

    #[test]
    fn test_ws_product_name_data() {
        let json_default_build_config = r#"
        {
            "version": "6",
            "name": "test-name",
            "product": "test-product",
            "description": "test description",
            "arch": "test-arch"
        }"#;
        let data: WsProductData = WsProductData::from_str(json_default_build_config)
            .expect("Failed to parse product data");
        assert_eq!(data.name(), "test-name");
        assert_eq!(data.project(), data.name());
        assert_eq!(data.product(), "test-product");
        assert_eq!(data.description(), "test description");
        assert_eq!(data.arch(), "test-arch");
    }
}

use dicom_object::InMemDicomObject;

// #[cfg(target_arch = "wasm32")]
use paste::*;

#[macro_export]
macro_rules! define_dicom_struct {
    // Main macro to define a struct with fields, types, DICOM tags, and optionality
    ($name:ident, { $(($field_name:ident, $field_type:ty, $dicom_tag:expr, $is_optional:tt)),* $(,)? }) => {
        // #[cfg_attr(target_arch = "wasm32", wasm_bindgen)] // Allow use in WASM
        #[derive(Debug, Clone, serde::Serialize)]
        pub struct $name {
            // Generate struct fields based on optionality
            $(
                // #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
                pub $field_name: $crate::define_dicom_struct!(@optional $field_type, $is_optional),
            )*
        }


        // #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
        impl $name {
            // Constructor function to create struct instances
            // #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
            // #[cfg(not(target_arch = "wasm32"))]
            pub fn new($($field_name: $crate::define_dicom_struct!(@constructor_type $field_type, $is_optional)),*) -> Self {
                $name {
                    $(
                        $field_name,
                    )*
                }
            }

            // Function to format DICOM tags and their corresponding values into a String
            // #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            pub fn format_tags(&self) -> String {
                let mut result = String::new();
                $(
                    $crate::define_dicom_struct!(@to_string $field_name, $field_type, $dicom_tag, $is_optional, self, result);
                )*
                result
            }
        }
        // paste::item!{
        //     #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
        //     #[cfg(target_arch = "wasm32")]
        //     impl $name {
        //         // Generate getters and setters for each field
        //         $(
        //             #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
        //             pub fn $field_name(&self) -> $crate::define_dicom_struct!(@getter_type $field_type, $is_optional) {
        //                 $crate::define_dicom_struct!(@getter_impl &self.$field_name, $is_optional)
        //             }

        //             #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
        //             pub fn [< set_ $field_name >] (&mut self, value: $crate::define_dicom_struct!(@setter_type $field_type, $is_optional)) {
        //                 self.$field_name = value;
        //             }
        //         )*
        //     }
        // }
    };

    // Helper rule to wrap type in Option if the field is optional
    (@optional $field_type:ty, true) => {
        Option<$field_type>
    };
    (@optional $field_type:ty, false) => {
        $field_type
    };

    // Helper rule for constructor argument types
    (@constructor_type $field_type:ty, true) => {
        Option<$field_type>
    };
    (@constructor_type $field_type:ty, false) => {
        $field_type
    };

    // Helper rule for getter return types
    (@getter_type $field_type:ty, true) => {
        Option<$field_type>
    };
    (@getter_type $field_type:ty, false) => {
        $field_type
    };

    // // Helper rule for getter implementation
    // (@getter_impl $field:expr, true) => {
    //     $field.clone()
    // };
    // (@getter_impl $field:expr, false) => {
    //     $field.clone()
    // };

    // // Helper rule for setter argument types
    // (@setter_type $field_type:ty, true) => {
    //     Option<$field_type>
    // };
    // (@setter_type $field_type:ty, false) => {
    //     $field_type
    // };

    // Helper rule to handle formatting for optional fields
    (@to_string $field_name:ident, $field_type:ty, $dicom_tag:expr, true, $self:ident, $result:ident) => {
        let value = match &$self.$field_name {
            Some(val) => format!("{}: Some({:?})\n", $dicom_tag, val),
            None => format!("{}: None (Optional)\n", $dicom_tag),
        };
        $result.push_str(value.as_str());
    };

    // Helper rule to handle formatting for mandatory fields
    (@to_string $field_name:ident, $field_type:ty, $dicom_tag:expr, false, $self:ident, $result:ident) => {
        $result.push_str(format!("{}: {:?}\n", $dicom_tag, &$self.$field_name).as_str());
    };
}

// Helper function to safely retrieve a tag value and convert it to a type T
pub fn get_value<T>(obj: &InMemDicomObject, tag: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    obj.element_by_name(tag)
        .ok()
        .and_then(|e| e.value().to_str().ok())
        .and_then(|v| v.parse::<T>().ok())
}

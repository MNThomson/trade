use hypertext::{Attribute, GlobalAttributes};

pub trait HtmxAttributes: GlobalAttributes {
    #![allow(non_upper_case_globals)]
    const hx_get: Attribute = Attribute;
    const hx_post: Attribute = Attribute;
}

impl<T: GlobalAttributes> HtmxAttributes for T {}

#[cfg(test)]
pub mod tests {
    use hypertext::{Renderable, html_elements, rsx};
    use pretty_assertions::assert_eq;

    use super::HtmxAttributes;

    #[test]
    fn test_rsx_macro_with_htmx_attributes() {
        let component = rsx! {
            <div hx-get="" hx-post=""></div>
        }
        .render()
        .0;

        assert_eq!(r#"<div hx-get="" hx-post=""></div>"#, component);
    }
}

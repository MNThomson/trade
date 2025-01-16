pub use hypertext::{
    Attribute, GlobalAttributes, Raw, RenderIterator, Renderable, Rendered, VoidElement,
    html_elements, rsx, rsx_move, rsx_static,
};

pub trait HtmxAttributes: GlobalAttributes {
    #![allow(non_upper_case_globals)]
    const hx_get: Attribute = Attribute;
    const hx_post: Attribute = Attribute;
}

impl<T: GlobalAttributes> HtmxAttributes for T {}

#[cfg(test)]
pub mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

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

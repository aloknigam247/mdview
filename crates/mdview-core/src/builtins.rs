use crate::ext::MdViewExtension;

pub fn builtin_extensions() -> Vec<Box<dyn MdViewExtension>> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_start_empty() {
        assert!(builtin_extensions().is_empty());
    }
}

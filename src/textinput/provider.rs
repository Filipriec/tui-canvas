use crate::DataProvider;

fn sanitize_single_line(text: String) -> String {
    text.chars()
        .take_while(|&ch| ch != '\n' && ch != '\r')
        .collect()
}

pub trait TextInputDataProvider: DataProvider {
    fn from_text(text: String) -> Self
    where
        Self: Sized;

    fn to_text(&self) -> String;
    fn set_text(&mut self, text: String);
}

#[derive(Debug, Clone)]
pub struct TextInputProvider {
    value: String,
    name: String,
}

impl Default for TextInputProvider {
    fn default() -> Self {
        Self {
            value: String::new(),
            name: "Input".to_string(),
        }
    }
}

impl TextInputProvider {
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        Self {
            value: sanitize_single_line(text.into()),
            name: "Input".to_string(),
        }
    }

    pub fn to_text(&self) -> String {
        self.value.clone()
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        self.value = sanitize_single_line(text.into());
    }
}

impl DataProvider for TextInputProvider {
    fn field_count(&self) -> usize {
        1
    }

    fn field_name(&self, _index: usize) -> &str {
        &self.name
    }

    fn field_value(&self, index: usize) -> &str {
        if index == 0 {
            &self.value
        } else {
            ""
        }
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        if index == 0 {
            self.value = sanitize_single_line(value);
        }
    }
}

impl TextInputDataProvider for TextInputProvider {
    fn from_text(text: String) -> Self {
        Self::from_text(text)
    }

    fn to_text(&self) -> String {
        TextInputProvider::to_text(self)
    }

    fn set_text(&mut self, text: String) {
        TextInputProvider::set_text(self, text);
    }
}

#[cfg(test)]
mod tests {
    use super::TextInputProvider;
    use crate::DataProvider;

    #[test]
    fn strips_newlines_from_initial_text() {
        let provider = TextInputProvider::from_text("hello\nworld");
        assert_eq!(provider.field_value(0), "hello");
    }

    #[test]
    fn strips_newlines_on_assignment() {
        let mut provider = TextInputProvider::default();
        provider.set_field_value(0, "abc\r\ndef".to_string());
        assert_eq!(provider.field_value(0), "abc");
    }
}

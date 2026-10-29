use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Application {
    pub program: String,
    pub arguments: Vec<String>,
}

impl Application {
    pub fn new(program: String, arguments: Vec<String>) -> Result<Self, String> {
        if program.trim().is_empty() {
            return Err("file association application must not be empty".into());
        }

        Ok(Self { program, arguments })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileAssociations {
    by_extension: HashMap<String, Application>,
    text_editor: Option<Application>,
}

impl FileAssociations {
    pub fn set(
        &mut self,
        extensions: impl IntoIterator<Item = String>,
        application: Application,
    ) -> Result<(), String> {
        let extensions = extensions
            .into_iter()
            .map(|extension| normalize_extension(&extension))
            .collect::<Result<Vec<_>, _>>()?;
        if extensions.is_empty() {
            return Err("file association requires at least one extension".into());
        }

        for extension in extensions {
            self.by_extension.insert(extension, application.clone());
        }

        Ok(())
    }

    pub fn remove(&mut self, extensions: impl IntoIterator<Item = String>) -> Result<(), String> {
        for extension in extensions {
            self.by_extension.remove(&normalize_extension(&extension)?);
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.by_extension.clear();
        self.text_editor = None;
    }

    pub fn set_text_editor(&mut self, application: Application) {
        self.text_editor = Some(application);
    }

    pub fn clear_text_editor(&mut self) {
        self.text_editor = None;
    }

    pub fn text_editor(&self) -> Option<&Application> {
        self.text_editor.as_ref()
    }

    pub fn application_for(&self, path: &Path) -> Option<&Application> {
        let file_name = path.file_name()?.to_string_lossy().to_lowercase();

        self.by_extension
            .iter()
            .filter(|(extension, _)| file_name.ends_with(&format!(".{extension}")))
            .max_by_key(|(extension, _)| extension.len())
            .map(|(_, application)| application)
    }
}

fn normalize_extension(extension: &str) -> Result<String, String> {
    let extension = extension.trim().trim_start_matches('.').to_lowercase();
    if extension.is_empty()
        || extension.starts_with('.')
        || extension.ends_with('.')
        || extension.contains(['/', '\\'])
    {
        return Err(format!("invalid file association extension: '{extension}'"));
    }

    Ok(extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn application(name: &str) -> Application {
        Application::new(name.into(), Vec::new()).unwrap()
    }

    #[test]
    fn associations_accept_lists_and_prefer_the_longest_matching_suffix() {
        let mut associations = FileAssociations::default();
        associations
            .set(["jpg".into(), ".JPEG".into()], application("images"))
            .unwrap();
        associations
            .set(["tar.gz".into()], application("archives"))
            .unwrap();
        associations
            .set(["gz".into()], application("compression"))
            .unwrap();

        assert_eq!(
            associations
                .application_for(Path::new("Wolf.JpEg"))
                .unwrap()
                .program,
            "images"
        );
        assert_eq!(
            associations
                .application_for(Path::new("backup.TAR.GZ"))
                .unwrap()
                .program,
            "archives"
        );
        associations.set_text_editor(application("editor"));
        assert_eq!(associations.text_editor().unwrap().program, "editor");
        associations.clear_text_editor();
        assert!(associations.text_editor().is_none());
    }
}

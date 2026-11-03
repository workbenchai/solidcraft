//! Native project editor. Drafts never replace a concurrent document revision silently.
use crate::{SolveApp, ui_kit};
use serde_json::{Value, json};
use solidcraft_engine::doc::ebike::Project;
use std::collections::BTreeMap;

pub struct Editor {
    revision: u64,
    document_id: String,
    design: String,
    files: BTreeMap<String, String>,
    path: String,
    source: String,
    report: String,
    tab: usize,
    error: Option<String>,
    pending: bool,
}
impl Editor {
    fn load(app: &mut SolveApp) -> Self {
        let project = app.session.doc.ebike.clone().unwrap_or_default();
        let mut value = serde_json::to_value(&project).unwrap_or(Value::Null);
        if let Some(object) = value.as_object_mut() {
            object.remove("files");
        }
        let path = project.files.keys().next().cloned().unwrap_or_default();
        let source = project.files.get(&path).cloned().unwrap_or_default();
        let report = app.session.execute("ebike.inspect", &json!({})).ok().and_then(|v| serde_json::to_string_pretty(&v).ok()).unwrap_or_default();
        Self {
            revision: app.session.revision,
            document_id: app.session.document_id.clone(),
            design: serde_json::to_string_pretty(&value).unwrap_or_default(),
            files: project.files,
            path,
            source,
            report,
            tab: 0,
            error: None,
            pending: false,
        }
    }
    fn project(&mut self) -> Result<Project, String> {
        if !self.path.is_empty() {
            self.files.insert(self.path.clone(), self.source.clone());
        }
        let mut value: Value = serde_json::from_str(&self.design).map_err(|e| e.to_string())?;
        let object = value.as_object_mut().ok_or_else(|| "Project definition must be an object".to_string())?;
        object.insert("files".into(), json!(self.files));
        serde_json::from_value(value).map_err(|e| e.to_string())
    }
}
pub fn open(app: &mut SolveApp) {
    app.ebike_editor = Some(Editor::load(app));
}
pub fn show(app: &mut SolveApp, ctx: &egui::Context) {
    let Some(mut editor) = app.ebike_editor.take() else { return };
    // A different document must never receive an old editor's draft.
    if editor.document_id != app.session.document_id {
        return;
    }
    if editor.pending && app.computing_since().is_none() {
        if editor.revision != app.session.revision {
            editor = Editor::load(app);
        } else {
            editor.pending = false;
        }
    }
    let exists = app.session.doc.ebike.is_some();
    let mut apply = false;
    let mut reload = false;
    let closed =
        ui_kit::Window::new(egui::Id::new("ebike_project"), "component", crate::i18n::tr("ui.ebike.title")).cards(3.0).show_body(ctx, |ui| {
            ui.label(crate::i18n::tr("ui.ebike.study"));
            let labels = [crate::i18n::tr("ui.ebike.design"), crate::i18n::tr("ui.ebike.sources"), crate::i18n::tr("ui.ebike.checks")];
            ui_kit::segmented(ui, &mut editor.tab, &labels, Default::default());
            ui.separator();
            match editor.tab {
                0 => {
                    ui.add(
                        egui::TextEdit::multiline(&mut editor.design).char_limit(65536).code_editor().desired_rows(18).desired_width(f32::INFINITY),
                    );
                }
                1 => {
                    let paths: Vec<_> = editor.files.keys().cloned().collect();
                    ui.horizontal_wrapped(|ui| {
                        for path in paths {
                            if ui_kit::text_button(ui, &path, true).clicked() {
                                editor.files.insert(editor.path.clone(), editor.source.clone());
                                editor.source = editor.files.get(&path).cloned().unwrap_or_default();
                                editor.path = path;
                            }
                        }
                    });
                    ui.label(&editor.path);
                    ui.add(
                        egui::TextEdit::multiline(&mut editor.source).char_limit(65536).code_editor().desired_rows(18).desired_width(f32::INFINITY),
                    );
                }
                _ => {
                    ui.add(
                        egui::TextEdit::multiline(&mut editor.report).code_editor().interactive(false).desired_rows(18).desired_width(f32::INFINITY),
                    );
                }
            }
            if let Some(error) = &editor.error {
                ui.label(error);
            }
            ui.horizontal(|ui| {
                let label = crate::i18n::tr(if exists { "ui.ebike.apply" } else { "ui.ebike.create" });
                apply = ui_kit::text_button(ui, label, !editor.pending).clicked();
                reload = ui_kit::text_button(ui, crate::i18n::tr("ui.ebike.reload"), !editor.pending).clicked();
            });
        });
    if apply {
        let result = editor.project().and_then(|project| {
            if exists {
                app.run_heavy("ebike.configure", json!({"project":project,"expected_revision":editor.revision}))
            } else {
                app.run_heavy("ebike.create", json!({"project":project}))
            }
        });
        match result {
            Ok(Value::Null) => {
                editor.pending = true;
            }
            Ok(_) => {
                editor = Editor::load(app);
            }
            Err(error) => {
                editor.error = Some(error);
            }
        }
    }
    if reload {
        editor = Editor::load(app);
    }
    if !closed {
        app.ebike_editor = Some(editor);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    #[test]
    fn draft_keeps_sources_and_does_not_create_a_design() {
        let mut app = SolveApp::new(Default::default(), Default::default());
        let mut editor = Editor::load(&mut app);
        editor.source = "edited controller source".into();
        let path = editor.path.clone();
        let project = editor.project().unwrap();
        assert_eq!(project.files.get(&path).map(String::as_str), Some("edited controller source"));
        assert!(app.session.doc.ebike.is_none());
        assert!(app.session.doc.features.is_empty());
    }
    #[test]
    fn another_document_discards_the_old_editor() {
        let mut app = SolveApp::new(Default::default(), Default::default());
        open(&mut app);
        app.session.execute("file.new", &json!({})).unwrap();
        show(&mut app, &egui::Context::default());
        assert!(app.ebike_editor.is_none());
    }
}

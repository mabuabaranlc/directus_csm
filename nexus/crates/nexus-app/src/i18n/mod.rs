use std::collections::HashMap;
use leptos::prelude::*;

/// Simple i18n system using key-value translation maps
#[derive(Clone, Debug)]
pub struct I18n {
    locale: RwSignal<String>,
    translations: HashMap<String, HashMap<String, String>>,
}

impl I18n {
    pub fn new() -> Self {
        let mut translations = HashMap::new();

        // English translations (default)
        let mut en = HashMap::new();
        en.insert("app.title".into(), "Nexus CMS".into());
        en.insert("auth.login".into(), "Sign In".into());
        en.insert("auth.logout".into(), "Sign Out".into());
        en.insert("auth.email".into(), "Email".into());
        en.insert("auth.password".into(), "Password".into());
        en.insert("nav.content".into(), "Content".into());
        en.insert("nav.users".into(), "User Directory".into());
        en.insert("nav.files".into(), "File Library".into());
        en.insert("nav.insights".into(), "Insights".into());
        en.insert("nav.settings".into(), "Settings".into());
        en.insert("nav.activity".into(), "Activity Log".into());
        en.insert("actions.create".into(), "Create".into());
        en.insert("actions.save".into(), "Save".into());
        en.insert("actions.delete".into(), "Delete".into());
        en.insert("actions.cancel".into(), "Cancel".into());
        en.insert("actions.search".into(), "Search...".into());
        en.insert("empty.no_items".into(), "No Items".into());
        en.insert("empty.no_results".into(), "No Results".into());
        en.insert("pagination.page_of".into(), "Page {page} of {total}".into());
        translations.insert("en-US".into(), en);

        // Spanish translations
        let mut es = HashMap::new();
        es.insert("app.title".into(), "Nexus CMS".into());
        es.insert("auth.login".into(), "Iniciar Sesión".into());
        es.insert("auth.logout".into(), "Cerrar Sesión".into());
        es.insert("auth.email".into(), "Correo electrónico".into());
        es.insert("auth.password".into(), "Contraseña".into());
        es.insert("nav.content".into(), "Contenido".into());
        es.insert("nav.users".into(), "Directorio de Usuarios".into());
        es.insert("nav.files".into(), "Biblioteca de Archivos".into());
        es.insert("nav.insights".into(), "Estadísticas".into());
        es.insert("nav.settings".into(), "Configuración".into());
        es.insert("nav.activity".into(), "Registro de Actividad".into());
        es.insert("actions.create".into(), "Crear".into());
        es.insert("actions.save".into(), "Guardar".into());
        es.insert("actions.delete".into(), "Eliminar".into());
        es.insert("actions.cancel".into(), "Cancelar".into());
        es.insert("actions.search".into(), "Buscar...".into());
        es.insert("empty.no_items".into(), "Sin elementos".into());
        es.insert("empty.no_results".into(), "Sin resultados".into());
        es.insert("pagination.page_of".into(), "Página {page} de {total}".into());
        translations.insert("es-ES".into(), es);

        Self {
            locale: RwSignal::new("en-US".into()),
            translations,
        }
    }

    /// Get a translation for the current locale
    pub fn t(&self, key: &str) -> String {
        let locale = self.locale.get();
        self.translations
            .get(&locale)
            .and_then(|t| t.get(key))
            .or_else(|| {
                self.translations.get("en-US").and_then(|t| t.get(key))
            })
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Set the current locale
    pub fn set_locale(&self, locale: &str) {
        self.locale.set(locale.to_string());
    }

    /// Get the current locale
    pub fn locale(&self) -> String {
        self.locale.get()
    }

    /// Get available locales
    pub fn available_locales(&self) -> Vec<String> {
        self.translations.keys().cloned().collect()
    }
}

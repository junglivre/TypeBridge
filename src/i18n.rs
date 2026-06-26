//! Minimal compile-time internationalization.
//!
//! Each [`Lang`] maps to a static [`Strings`] table. No runtime lookup cost,
//! no external files.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lang {
    En,
    PtBr,
    Es,
}

impl Default for Lang {
    fn default() -> Self {
        Lang::En
    }
}

impl Lang {
    /// All languages, in selector order.
    pub const ALL: [Lang; 3] = [Lang::En, Lang::PtBr, Lang::Es];

    /// Native display name for the language selector.
    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::PtBr => "Português (BR)",
            Lang::Es => "Español",
        }
    }

    /// The string table for this language.
    pub fn s(self) -> &'static Strings {
        match self {
            Lang::En => &EN,
            Lang::PtBr => &PT_BR,
            Lang::Es => &ES,
        }
    }
}

/// All user-visible UI strings.
pub struct Strings {
    pub subtitle: &'static str,
    pub language: &'static str,
    pub text: &'static str,
    pub hint: &'static str,
    pub paste_clipboard: &'static str,
    pub clear: &'static str,
    pub characters: &'static str,
    pub delay_between_keys: &'static str,
    pub initial_delay: &'static str,
    pub initial_delay_help: &'static str,
    pub presets: &'static str,
    pub very_fast: &'static str,
    pub fast: &'static str,
    pub normal: &'static str,
    pub slow: &'static str,
    pub very_slow: &'static str,
    pub minimize_before: &'static str,
    pub detect_window_change: &'static str,
    pub detect_window_change_help: &'static str,
    pub physical_keys: &'static str,
    pub physical_keys_help: &'static str,
    pub start_typing: &'static str,
    pub cancel: &'static str,
    pub status: &'static str,
    pub ready: &'static str,
    pub waiting: &'static str,
    pub typing: &'static str,
    pub finished: &'static str,
    pub cancelled: &'static str,
    pub error: &'static str,
    pub paused: &'static str,
    pub no_text: &'static str,
    pub clipboard_error: &'static str,
    pub window_changed_title: &'static str,
    pub window_changed_msg: &'static str,
    pub continue_btn: &'static str,
    pub restart_btn: &'static str,
}

static EN: Strings = Strings {
    subtitle: "Types text into the focused window — no clipboard, no paste.",
    language: "Language",
    text: "Text",
    hint: "Type or paste the text to send...",
    paste_clipboard: "Paste Clipboard",
    clear: "Clear",
    characters: "characters",
    delay_between_keys: "Delay between keys",
    initial_delay: "Initial delay",
    initial_delay_help: "(time to switch to the target window)",
    presets: "Presets:",
    very_fast: "Very fast",
    fast: "Fast",
    normal: "Normal",
    slow: "Slow",
    very_slow: "Very slow",
    minimize_before: "Minimize window before typing",
    detect_window_change: "Stop if the focused window changes",
    detect_window_change_help: "(pauses and warns if focus leaves the target)",
    physical_keys: "Physical keys (best for VNC/RDP/remote)",
    physical_keys_help: "(sends real keypresses with Shift; turn off only for special characters in local apps)",
    start_typing: "▶  Start Typing",
    cancel: "Cancel (Esc)",
    status: "Status:",
    ready: "Ready",
    waiting: "Waiting…",
    typing: "Typing…",
    finished: "Finished",
    cancelled: "Cancelled",
    error: "Error",
    paused: "Paused",
    no_text: "No text to type.",
    clipboard_error: "Could not read the clipboard.",
    window_changed_title: "Focus changed — typing paused",
    window_changed_msg:
        "The active window changed. Refocus the target window and continue, \
         or restart to reconfigure.",
    continue_btn: "Continue",
    restart_btn: "Restart",
};

static PT_BR: Strings = Strings {
    subtitle: "Digita texto na janela em foco — sem área de transferência, sem colar.",
    language: "Idioma",
    text: "Texto",
    hint: "Digite ou cole o texto a enviar...",
    paste_clipboard: "Colar da Área de Transferência",
    clear: "Limpar",
    characters: "caracteres",
    delay_between_keys: "Atraso entre teclas",
    initial_delay: "Atraso inicial",
    initial_delay_help: "(tempo para mudar para a janela de destino)",
    presets: "Predefinições:",
    very_fast: "Muito rápido",
    fast: "Rápido",
    normal: "Normal",
    slow: "Lento",
    very_slow: "Muito lento",
    minimize_before: "Minimizar janela antes de digitar",
    detect_window_change: "Parar se a janela em foco mudar",
    detect_window_change_help: "(pausa e avisa se o foco sair do destino)",
    physical_keys: "Teclas físicas (melhor para VNC/RDP/remoto)",
    physical_keys_help: "(envia teclas reais com Shift; desligue só para caracteres especiais em apps locais)",
    start_typing: "▶  Iniciar Digitação",
    cancel: "Cancelar (Esc)",
    status: "Status:",
    ready: "Pronto",
    waiting: "Aguardando…",
    typing: "Digitando…",
    finished: "Concluído",
    cancelled: "Cancelado",
    error: "Erro",
    paused: "Pausado",
    no_text: "Nenhum texto para digitar.",
    clipboard_error: "Não foi possível ler a área de transferência.",
    window_changed_title: "Foco mudou — digitação pausada",
    window_changed_msg:
        "A janela ativa mudou. Volte o foco para a janela de destino e continue, \
         ou recomece para reconfigurar.",
    continue_btn: "Continuar",
    restart_btn: "Recomeçar",
};

static ES: Strings = Strings {
    subtitle: "Escribe texto en la ventana enfocada — sin portapapeles, sin pegar.",
    language: "Idioma",
    text: "Texto",
    hint: "Escribe o pega el texto a enviar...",
    paste_clipboard: "Pegar del Portapapeles",
    clear: "Limpiar",
    characters: "caracteres",
    delay_between_keys: "Retardo entre teclas",
    initial_delay: "Retardo inicial",
    initial_delay_help: "(tiempo para cambiar a la ventana de destino)",
    presets: "Preajustes:",
    very_fast: "Muy rápido",
    fast: "Rápido",
    normal: "Normal",
    slow: "Lento",
    very_slow: "Muy lento",
    minimize_before: "Minimizar ventana antes de escribir",
    detect_window_change: "Detener si cambia la ventana enfocada",
    detect_window_change_help: "(pausa y avisa si el foco sale del destino)",
    physical_keys: "Teclas físicas (mejor para VNC/RDP/remoto)",
    physical_keys_help: "(envía pulsaciones reales con Shift; desactívalo solo para caracteres especiales en apps locales)",
    start_typing: "▶  Empezar a Escribir",
    cancel: "Cancelar (Esc)",
    status: "Estado:",
    ready: "Listo",
    waiting: "Esperando…",
    typing: "Escribiendo…",
    finished: "Finalizado",
    cancelled: "Cancelado",
    error: "Error",
    paused: "Pausado",
    no_text: "No hay texto para escribir.",
    clipboard_error: "No se pudo leer el portapapeles.",
    window_changed_title: "El foco cambió — escritura pausada",
    window_changed_msg:
        "La ventana activa cambió. Vuelve a enfocar la ventana de destino y continúa, \
         o reinicia para reconfigurar.",
    continue_btn: "Continuar",
    restart_btn: "Reiniciar",
};

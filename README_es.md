# TypeBridge

[![CI](https://github.com/junglivre/TypeBridge/actions/workflows/ci.yml/badge.svg)](https://github.com/junglivre/TypeBridge/actions/workflows/ci.yml)
[![Última versión](https://img.shields.io/github/v/release/junglivre/TypeBridge?sort=semver)](https://github.com/junglivre/TypeBridge/releases/latest)
[![Licencia](https://img.shields.io/badge/licencia-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)

- [English](README.md)
- [Português](README_br.md)
- **Español**

Una utilidad ligera y multiplataforma que **escribe texto en la ventana que
tiene el foco** — una pulsación a la vez. Pensada para VNC, Guacamole, KVMs,
consolas remotas, terminales web y entornos BIOS/IPMI donde no hay portapapeles
compartido.

*Simula entradas de teclado reales*. **No** pega y **no** envía el portapapeles.

- Nativa (Rust + [egui]/[eframe]), sin runtime de Electron/Java/Python/.NET
- Binario nativo pequeño, arranque rápido, poca memoria
- Sin telemetría, sin cuenta — funciona 100% sin conexión (la única llamada de
  red es una comprobación de actualización opcional y silenciosa)

## Descarga

Hay binarios listos para **Windows, Linux y macOS** adjuntos en cada
[release](https://github.com/junglivre/TypeBridge/releases) (generados
automáticamente por GitHub Actions). O compila desde el código — ve
[Compilación](#compilación).

---

## Características

- **Editor multilínea compatible con Unicode** — los tabuladores y saltos de
  línea se convierten en pulsaciones reales `Tab` / `Enter`.
- **Retardo por tecla configurable** (1–2000 ms) y **retardo inicial** (tiempo
  para cambiar a la ventana de destino).
- **Preajustes de velocidad** (Muy rápido → Muy lento).
- **Modo de teclas físicas** *(predeterminado)* — escribe con pulsaciones reales
  y los modificadores `Shift`/`Ctrl`/`Alt` correctos, para que `#`, mayúsculas y
  símbolos lleguen bien en **VNC/RDP/KVM y consolas web** (noVNC, Guacamole…).
  Desactívalo para inyección Unicode (p. ej. caracteres especiales en apps
  locales).
- **Interfaz multilingüe** — Inglés, Português (BR) y Español, conmutable en
  tiempo de ejecución.
- **Linux: X11 *y* Wayland** — funciona en ambos; el backend de Wayland correcto
  (wlroots / GNOME / KDE) se elige automáticamente. Ve
  [cómo escribe](#cómo-escribe-typebridge).
- **Comprobación de actualización integrada** — consulta silenciosamente GitHub
  por una versión nueva al iniciar y, si existe, muestra un **popup con el
  changelog** (renderizado en Markdown) y un enlace de descarga (sin telemetría;
  solo un ping de versión).
- **Guardia de foco** *(opcional)* — si la ventana enfocada cambia durante la
  escritura (una notificación roba el foco, cambias de ventana sin querer…), la
  escritura se **pausa**, la ventana pasa al frente (y parpadea en la barra de
  tareas) y una alerta modal destacada te permite **continuar** (con una nueva
  cuenta regresiva para volver a enfocar el destino) o **reiniciar** y
  reconfigurar. *(Windows; sin efecto en otras plataformas.)*
- **Opción "minimizar antes de escribir"** para que la app se quite de en medio.
- **Cancela en cualquier momento con `Esc`** — funciona incluso minimizada (se
  vigila la tecla física) o con el botón Cancelar.
- Botón **Pegar del Portapapeles** (rellena el editor; nunca escribe solo).
- **Estado en vivo** (`Listo` / `Esperando…` / `Escribiendo…` / `Pausado` /
  `Finalizado` / `Cancelado`) con barra de progreso.
- **Persistencia de ajustes** (retardo, retardo inicial, minimizar, guardia de
  foco, modo de escritura, idioma, tamaño de ventana) con **modo portátil** como
  alternativa.
- **Hilo de escritura en segundo plano** — la interfaz nunca se congela.
- **CLI mínima** para precargar texto/parámetros.

---

## Uso

1. Escribe o pega el texto en el editor (o pulsa **Pegar del Portapapeles**).
2. Ajusta el **retardo** por tecla y el **retardo inicial**.
3. (Opcional) marca **Minimizar ventana antes de escribir**.
4. Pulsa **Empezar a Escribir** y enfoca la ventana de destino durante la cuenta
   regresiva.
5. Pulsa **`Esc`** en cualquier momento para detener de inmediato.

### Línea de comandos

Todas las opciones son opcionales:

```
typebridge --delay 25 --wait 5 --file notes.txt --minimize --autostart
```

| Opción           | Significado                                              |
|------------------|---------------------------------------------------------|
| `--delay <ms>`   | Retardo por tecla (1–2000)                               |
| `--wait <s>`     | Retardo inicial antes de empezar a escribir             |
| `--file <path>`  | Precarga el editor con un archivo de texto              |
| `--text <str>`   | Precarga el editor con una cadena literal               |
| `--minimize`     | Minimiza antes de escribir (`--no-minimize` lo desactiva)|
| `--autostart`    | Empieza a escribir automáticamente al abrir             |

### Ubicación de los ajustes

- **Modo portátil:** si existe un archivo `typebridge.toml` *junto al
  ejecutable*, se usa ese.
- En caso contrario, se usa el directorio de configuración del usuario del
  sistema operativo (vía [`confy`]).

---

## Compilación

Requiere un toolchain estable de Rust (`rustc`/`cargo`). Luego:

```sh
cargo build --release
# binario: target/release/typebridge(.exe)
cargo test          # ejecuta las pruebas unitarias
```

### Nota sobre el toolchain de Windows (importante)

Hay dos toolchains en Windows:

- **MSVC (recomendado, el más simple):** instala las *Visual Studio Build Tools*
  (carga de trabajo C++) y ejecuta `rustup default stable-msvc`. Sin
  configuración extra — compila directamente.

- **GNU (`x86_64-pc-windows-gnu`):** el MinGW **incluido con rustup es mínimo** y
  **no puede enlazar la pila completa de eframe/glow** — el binario resultante
  se cae con `STATUS_ACCESS_VIOLATION` *antes de que se ejecute `main`*.
  Necesitas un **MinGW-w64 completo** (p. ej. [WinLibs]):

  1. Descarga una build GCC de WinLibs y descomprímela (este repo usa
     `D:\mingw64`).
  2. Añade su directorio `bin` al `PATH` (aporta `gcc`, `as`, `dlltool`).
  3. Crea un `.cargo/config.toml` **local e ignorado por git** apuntando rustc a
     él:

     ```toml
     [target.x86_64-pc-windows-gnu]
     linker = 'D:\mingw64\bin\gcc.exe'
     rustflags = [
       '-Clink-self-contained=no',
       '-Cdlltool=D:\mingw64\bin\dlltool.exe',
     ]
     ```

  > `.cargo/` está ignorado por git a propósito — contiene rutas específicas de
  > la máquina y no debe publicarse.

---

## Cómo escribe TypeBridge

Inyectar pulsaciones sintéticas es trivial en Windows y X11, pero un verdadero
laberinto en Wayland: cada compositor expone un mecanismo distinto — e
incompleto — y ninguno deja que una herramienta en segundo plano simplemente
diga "escribe este texto". TypeBridge detecta el entorno y elige el backend
correcto automáticamente en tiempo de ejecución:

| Entorno | Backend | Manejo de la distribución |
|---|---|---|
| **Windows** | input Win32 (vía [enigo]) | Unicode, o scancodes físicos con mapa US fijo para VNC |
| **macOS** | eventos CoreGraphics (vía [enigo]) | Unicode |
| **Linux · X11** | XTEST (vía enigo `x11rb`) | lo maneja X |
| **Linux · Wayland · wlroots** (Sway, Hyprland, river, niri…) | `zwp_virtual_keyboard` con **nuestro propio keymap** | independiente de la distribución — el keymap es nuestro |
| **Linux · Wayland · GNOME** | portal RemoteDesktop `NotifyKeyboardKeysym` | Mutter resuelve el keysym en la distribución activa |
| **Linux · Wayland · KDE** | libei (portal RemoteDesktop), inyección de keycodes | grupo de distribución activo leído del D-Bus de KDE |
| **Linux · Wayland · otros / sin portal** (p. ej. Cinnamon) | recae en X11 / XWayland | lo maneja X (solo apps XWayland) |

### Por qué Wayland necesita cuatro enfoques

Wayland prohíbe a propósito la inyección global de input que permite el XTEST de
X11, así que no hay una API única. Lo que hizo falta para que la escritura
funcione en todos lados:

- Los compositores **wlroots** implementan `zwp_virtual_keyboard`, que permite al
  cliente **subir su propio keymap**. Subimos un keymap US y escribimos contra
  él, así la salida es correcta *sin importar la distribución activa* y sin
  diálogo de permiso. (Se vuelve la distribución activa del seat un instante
  mientras escribe y luego revierte — una peculiaridad de wlroots, no un cambio
  permanente de configuración.)
- **GNOME** y **KDE** no soportan `zwp_virtual_keyboard`; la emulación de input
  solo está disponible vía el **portal RemoteDesktop** (con un diálogo de
  permiso):
  - **GNOME/Mutter** resuelve el *keysym* en la distribución activa por sí mismo,
    así que el `NotifyKeyboardKeysym` del portal sale correcto.
  - **KDE/KWin** inyecta *keycodes* y los decodifica con el **grupo de
    distribución activo** — pero, al ser un servicio en segundo plano, no puede
    decirnos cuál es (el evento de Wayland relevante nunca le llega). Así que
    TypeBridge lee la distribución activa del propio D-Bus de KDE
    (`org.kde.keyboard`) y resuelve el keycode en ese grupo. (El propio camino de
    keysym de KWin tenía el mismo punto ciego, corregido upstream solo a fines de
    2025.)
- **Compositores sin portal RemoteDesktop** (p. ej. el Wayland experimental de
  Cinnamon) recaen en el backend X11, que aún alcanza apps XWayland.

Nunca eliges el backend — todo es automático.

### Binarios Linux portables

Los binarios Linux publicados se compilan con
[`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild) apuntando a
**glibc 2.31**, y usan **rustls** para TLS (sin OpenSSL), así que un solo binario
funciona en una amplia gama de distribuciones (Ubuntu 20.04+, Debian 11+, Mint,
Fedora…) sin desajustes de versión de `libssl`/glibc.

## Notas por plataforma

- **Windows** — funciona de inmediato.
- **Linux** — **X11 y Wayland** son compatibles (ve
  [Cómo escribe TypeBridge](#cómo-escribe-typebridge)). Dependencias de build:
  paquetes dev de X11/xcb + xkbcommon + Wayland (el TLS es Rust puro, sin
  OpenSSL).
- **macOS** — concede el permiso de Accesibilidad en *Ajustes del Sistema →
  Privacidad y Seguridad → Accesibilidad*; la app muestra un mensaje claro si
  falta.

---

## Estructura del proyecto

```
src/
  main.rs              punto de entrada, parsing de CLI, arranque de ventana
  i18n.rs              traducciones en tiempo de compilación (en / pt-br / es)
  ui/    app.rs        aplicación egui + bucle de actualización
         widgets.rs    pequeños helpers de UI
  typing/engine.rs     motor de carácter → pulsación (Typer; enigo + Wayland)
         wayland/      backends Wayland de Linux (libei · portal keysym · vkbd)
         worker.rs     hilo de escritura + canal de estado
         cancel.rs     flag de cancelación + vigilante del Esc físico
         window.rs     detección de la ventana enfocada (guardia de foco)
  settings/config.rs   carga/guarda ajustes (confy + modo portátil)
  clipboard/clipboard.rs  lectura del portapapeles (arboard)
```

---

## Fuera de alcance

Grabación de macros, automatización del ratón, scripting, OCR, sincronización de
portapapeles o software de escritorio remoto. TypeBridge hace exactamente una
cosa bien: escribir texto en la ventana enfocada.

## Licencia

Licencia dual: MIT o Apache-2.0.

Hecho por [jung](https://jung.moe).

[egui]: https://github.com/emilk/egui
[eframe]: https://crates.io/crates/eframe
[enigo]: https://github.com/enigo-rs/enigo
[`confy`]: https://crates.io/crates/confy
[WinLibs]: https://winlibs.com/

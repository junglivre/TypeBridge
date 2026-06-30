# TypeBridge

[![CI](https://github.com/junglivre/TypeBridge/actions/workflows/ci.yml/badge.svg)](https://github.com/junglivre/TypeBridge/actions/workflows/ci.yml)
[![Última versão](https://img.shields.io/github/v/release/junglivre/TypeBridge?sort=semver)](https://github.com/junglivre/TypeBridge/releases/latest)
[![Licença](https://img.shields.io/badge/licen%C3%A7a-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)

- [English](README.md)
- **Português**
- [Español](README_es.md)

Um utilitário leve e multiplataforma que **digita texto na janela que está em
foco** — uma tecla de cada vez. Feito para VNC, Guacamole, KVMs, consoles
remotos, terminais web e ambientes BIOS/IPMI onde não há compartilhamento de
área de transferência.

Ele *simula entrada de teclado real*. Ele **não** cola e **não** envia a área de
transferência.

- Nativo (Rust + [egui]/[eframe]), sem runtime Electron/Java/Python/.NET
- Binário nativo pequeno, inicialização rápida, pouca memória
- Sem telemetria, sem conta — funciona 100% offline (a única chamada de rede é
  uma verificação de atualização opcional e silenciosa)

## Download

Binários prontos para **Windows, Linux e macOS** são anexados a cada
[release](https://github.com/junglivre/TypeBridge/releases) (gerados
automaticamente pelo GitHub Actions). Ou compile do código — veja
[Compilando](#compilando).

---

## Recursos

- **Editor multilinha com suporte a Unicode** — tabs e quebras de linha viram
  teclas `Tab` / `Enter` de verdade.
- **Atraso por tecla configurável** (1–2000 ms) e **atraso inicial** (tempo para
  trocar para a janela de destino).
- **Predefinições de velocidade** (Muito rápido → Muito lento).
- **Modo de teclas físicas** *(padrão)* — digita com pressionamentos reais e os
  modificadores `Shift`/`Ctrl`/`Alt` corretos, então `#`, letras maiúsculas e
  símbolos chegam certos em **VNC/RDP/KVM e consoles web** (noVNC, Guacamole…).
  Desligue para injeção Unicode (ex.: caracteres especiais em apps locais).
- **Interface multilíngue** — Inglês, Português (BR) e Español, trocável em
  tempo de execução.
- **Linux: X11 *e* Wayland** — funciona nos dois; o backend Wayland certo
  (wlroots / GNOME / KDE) é escolhido automaticamente. Veja
  [como ele digita](#como-o-typebridge-digita).
- **Verificação de atualização embutida** — checa silenciosamente o GitHub por
  uma versão nova ao iniciar e, se houver, mostra um **popup com o changelog**
  (renderizado em Markdown) e um link de download (sem telemetria; só um ping de
  versão).
- **Guarda de foco** *(opcional)* — se a janela em foco mudar durante a
  digitação (uma notificação rouba o foco, você troca de janela sem querer…), a
  digitação **pausa**, a janela vem para a frente (e pisca na barra de tarefas)
  e um alerta modal em destaque permite **continuar** (com uma nova contagem
  regressiva para você voltar o foco ao destino) ou **recomeçar** e
  reconfigurar. *(Windows; sem efeito nas outras plataformas.)*
- **Opção "minimizar antes de digitar"** para o app sair da frente.
- **Cancele a qualquer momento com `Esc`** — funciona até minimizado (a tecla
  física é monitorada) ou pelo botão Cancelar.
- Botão **Colar da Área de Transferência** (preenche o editor; nunca digita
  automaticamente).
- **Status ao vivo** (`Pronto` / `Aguardando…` / `Digitando…` / `Pausado` /
  `Concluído` / `Cancelado`) com barra de progresso.
- **Persistência das configurações** (atraso, atraso inicial, minimizar, guarda
  de foco, modo de digitação, idioma, tamanho da janela) com **modo portátil**
  como alternativa.
- **Thread de digitação em segundo plano** — a interface nunca trava.
- **CLI mínima** para pré-carregar texto/parâmetros.

---

## Como usar

1. Digite ou cole o texto no editor (ou clique em **Colar da Área de
   Transferência**).
2. Defina o **atraso** por tecla e o **atraso inicial**.
3. (Opcional) marque **Minimizar janela antes de digitar**.
4. Clique em **Iniciar Digitação** e foque a janela de destino durante a
   contagem regressiva.
5. Pressione **`Esc`** a qualquer momento para parar imediatamente.

### Linha de comando

Todas as opções são opcionais:

```
typebridge --delay 25 --wait 5 --file notes.txt --minimize --autostart
```

| Opção            | Significado                                              |
|------------------|---------------------------------------------------------|
| `--delay <ms>`   | Atraso por tecla (1–2000)                                |
| `--wait <s>`     | Atraso inicial antes de começar a digitar               |
| `--file <path>`  | Pré-carrega o editor com um arquivo de texto            |
| `--text <str>`   | Pré-carrega o editor com uma string literal             |
| `--minimize`     | Minimiza antes de digitar (`--no-minimize` desativa)    |
| `--autostart`    | Começa a digitar automaticamente ao abrir               |

### Local das configurações

- **Modo portátil:** se existir um arquivo `typebridge.toml` *ao lado do
  executável*, ele é usado.
- Caso contrário, é usado o diretório de configuração do usuário do sistema
  operacional (via [`confy`]).

---

## Compilando

Requer um toolchain Rust estável (`rustc`/`cargo`). Então:

```sh
cargo build --release
# binário: target/release/typebridge(.exe)
cargo test          # roda os testes unitários
```

### Nota sobre o toolchain do Windows (importante)

Existem dois toolchains no Windows:

- **MSVC (recomendado, mais simples):** instale o *Visual Studio Build Tools*
  (workload C++) e rode `rustup default stable-msvc`. Sem configuração extra —
  compila direto.

- **GNU (`x86_64-pc-windows-gnu`):** o MinGW **que vem embutido no rustup é
  mínimo** e **não consegue linkar a stack completa do eframe/glow** — o binário
  resultante crasha com `STATUS_ACCESS_VIOLATION` *antes do `main` rodar*. Você
  precisa de um **MinGW-w64 completo** (ex.: [WinLibs]):

  1. Baixe um build GCC do WinLibs e extraia (este repo usa `D:\mingw64`).
  2. Adicione o diretório `bin` ao seu `PATH` (fornece `gcc`, `as`, `dlltool`).
  3. Crie um `.cargo/config.toml` **local e git-ignored** apontando o rustc para
     ele:

     ```toml
     [target.x86_64-pc-windows-gnu]
     linker = 'D:\mingw64\bin\gcc.exe'
     rustflags = [
       '-Clink-self-contained=no',
       '-Cdlltool=D:\mingw64\bin\dlltool.exe',
     ]
     ```

  > O `.cargo/` é git-ignored de propósito — contém caminhos específicos da
  > máquina e não deve ser publicado.

---

## Como o TypeBridge digita

Injetar teclas sintéticas é trivial no Windows e no X11, mas um verdadeiro
labirinto no Wayland: cada compositor expõe um mecanismo diferente — e
incompleto — e nenhum deixa uma ferramenta de fundo simplesmente dizer "digite
esse texto". O TypeBridge detecta o ambiente e escolhe o backend certo
automaticamente em tempo de execução:

| Ambiente | Backend | Tratamento de layout |
|---|---|---|
| **Windows** | input Win32 (via [enigo]) | Unicode, ou scancodes físicos com mapa US fixo p/ VNC |
| **macOS** | eventos CoreGraphics (via [enigo]) | Unicode |
| **Linux · X11** | XTEST (via enigo `x11rb`) | tratado pelo X |
| **Linux · Wayland · wlroots** (Sway, Hyprland, river, niri…) | `zwp_virtual_keyboard` com **nosso próprio keymap** | independente de layout — o keymap é nosso |
| **Linux · Wayland · GNOME** | portal RemoteDesktop `NotifyKeyboardKeysym` | o Mutter resolve o keysym na layout ativa |
| **Linux · Wayland · KDE** | libei (portal RemoteDesktop), injeção de keycodes | grupo de layout ativo lido do D-Bus do KDE |
| **Linux · Wayland · outros / sem portal** (ex.: Cinnamon) | fallback p/ X11 / XWayland | tratado pelo X (só apps XWayland) |

### Por que o Wayland precisa de quatro abordagens

O Wayland proíbe de propósito a injeção global de input que o XTEST do X11
permite, então não existe uma API única. O que foi preciso pra fazer a
digitação funcionar em todo lugar:

- Compositores **wlroots** implementam o `zwp_virtual_keyboard`, que deixa o
  cliente **subir o próprio keymap**. A gente sobe um keymap US e digita contra
  ele, então a saída fica correta *independente da layout ativa* e sem diálogo
  de permissão. (Ele vira a layout ativa do seat por um instante durante a
  digitação e depois reverte — uma peculiaridade do wlroots, não uma mudança
  permanente de config.)
- **GNOME** e **KDE** não suportam o `zwp_virtual_keyboard`; a emulação de input
  só existe pelo **portal RemoteDesktop** (com um diálogo de permissão):
  - O **GNOME/Mutter** resolve o *keysym* na layout ativa sozinho, então o
    `NotifyKeyboardKeysym` do portal já sai correto.
  - O **KDE/KWin** injeta *keycodes* e os decodifica com o **grupo de layout
    ativo** — mas, sendo um serviço de fundo, ele não consegue nos dizer qual é
    esse grupo (o evento Wayland relevante nunca chega até ele). Então o
    TypeBridge lê a layout ativa do próprio D-Bus do KDE (`org.kde.keyboard`) e
    resolve o keycode nesse grupo. (O próprio caminho de keysym do KWin tinha o
    mesmo ponto cego, corrigido upstream só no fim de 2025.)
- **Compositores sem portal RemoteDesktop** (ex.: o Wayland experimental do
  Cinnamon) caem no backend X11, que ainda alcança apps XWayland.

Você nunca escolhe o backend — é tudo automático.

### Binários Linux portáteis

Os binários Linux publicados são compilados com
[`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild) mirando
**glibc 2.31**, e usam **rustls** pro TLS (sem OpenSSL), então um único binário
roda numa ampla gama de distribuições (Ubuntu 20.04+, Debian 11+, Mint, Fedora…)
sem incompatibilidades de versão de `libssl`/glibc.

## Notas por plataforma

- **Windows** — funciona de imediato.
- **Linux** — **X11 e Wayland** são suportados (veja
  [Como o TypeBridge digita](#como-o-typebridge-digita)). Dependências de build:
  pacotes dev de X11/xcb + xkbcommon + Wayland (o TLS é Rust puro, sem OpenSSL).
- **macOS** — conceda a permissão de Acessibilidade em *Ajustes do Sistema →
  Privacidade e Segurança → Acessibilidade*; o app mostra uma mensagem clara se
  ela estiver faltando.

---

## Estrutura do projeto

```
src/
  main.rs              ponto de entrada, parsing de CLI, bootstrap da janela
  i18n.rs              traduções em tempo de compilação (en / pt-br / es)
  ui/    app.rs        aplicação egui + loop de atualização
         widgets.rs    pequenos helpers de UI
  typing/engine.rs     motor de caractere → tecla (Typer; enigo + Wayland)
         wayland/      backends Wayland do Linux (libei · portal keysym · vkbd)
         worker.rs     thread de digitação + canal de status
         cancel.rs     flag de cancelamento + monitor do Esc físico
         window.rs     detecção da janela em foco (guarda de foco)
  settings/config.rs   carrega/salva configurações (confy + modo portátil)
  clipboard/clipboard.rs  leitura da área de transferência (arboard)
```

---

## Fora do escopo

Gravação de macros, automação de mouse, scripting, OCR, sincronização de área
de transferência ou software de acesso remoto. O TypeBridge faz exatamente uma
coisa bem: digitar texto na janela em foco.

## Licença

Licença dupla: MIT ou Apache-2.0.

Feito por [jung](https://jung.moe).

[egui]: https://github.com/emilk/egui
[eframe]: https://crates.io/crates/eframe
[enigo]: https://github.com/enigo-rs/enigo
[`confy`]: https://crates.io/crates/confy
[WinLibs]: https://winlibs.com/

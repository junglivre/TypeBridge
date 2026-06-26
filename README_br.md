# TypeBridge

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
- Binário de release minúsculo (~3,5 MB), inicialização rápida, pouca memória
- Sem telemetria, sem conta, sem necessidade de internet

---

## Recursos

- **Editor multilinha com suporte a Unicode** — tabs e quebras de linha viram
  teclas `Tab` / `Enter` de verdade.
- **Atraso por tecla configurável** (1–2000 ms) e **atraso inicial** (tempo para
  trocar para a janela de destino).
- **Predefinições de velocidade** (Muito rápido → Muito lento).
- **Interface multilíngue** — Inglês, Português (BR) e Español, trocável em
  tempo de execução.
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
  de foco, idioma, tamanho da janela) com **modo portátil** como alternativa.
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

## Notas por plataforma

- **Windows** — funciona de imediato.
- **Linux** — X11 é suportado; em sessões Wayland restritas a injeção de teclas
  pode não funcionar (você verá uma mensagem amigável). Dependências de build:
  um ambiente de desenvolvimento X11.
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
  typing/engine.rs     motor de caractere → tecla (Typer)
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

[egui]: https://github.com/emilk/egui
[eframe]: https://crates.io/crates/eframe
[`confy`]: https://crates.io/crates/confy
[WinLibs]: https://winlibs.com/

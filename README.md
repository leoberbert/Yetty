<p align="center">
  <img src="docs/images/yetty.svg" alt="Yetty" width="160">
</p>

<h1 align="center">Yetty</h1>

<p align="center">
  <strong>Todas as suas conexões em uma só janela.</strong><br>
  Um terminal gratuito e de código aberto, para Windows e Linux, que reúne SSH, SFTP, Telnet, porta serial e muito mais, com abas, sessões salvas e um visual limpo.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/plataforma-Windows-0078D6" alt="Windows">
  <img src="https://img.shields.io/badge/plataforma-Linux-FCC624" alt="Linux">
  <img src="https://img.shields.io/badge/linguagem-Rust-DEA584" alt="Rust">
  <img src="https://img.shields.io/badge/licen%C3%A7a-MIT-green" alt="MIT">
</p>

---

## Sumário

- [Por que o Yetty](#por-que-o-yetty)
- [Inspiração](#inspiração)
- [Visão geral](#visão-geral)
- [Download e instalação](#download-e-instalação)
- [Requisitos](#requisitos)
- [Compilar e executar](#compilar-e-executar)
- [Linux](#linux)
- [Primeiros passos](#primeiros-passos)
- [Telas](#telas)
- [Funcionalidades](#funcionalidades)
  - [Protocolos e conexões](#protocolos-e-conexões)
  - [Gerenciador de sessões](#gerenciador-de-sessões)
  - [Abas, painéis e layout](#abas-painéis-e-layout)
  - [Emulação de terminal e aparência](#emulação-de-terminal-e-aparência)
  - [Transferência de arquivos](#transferência-de-arquivos)
  - [Scripts e gravação](#scripts-e-gravação)
  - [Janela de comandos e Command Manager](#janela-de-comandos-e-command-manager)
  - [Chaves SSH e credenciais](#chaves-ssh-e-credenciais)
  - [Log, impressão e snapshots](#log-impressão-e-snapshots)
  - [Mapa de teclas](#mapa-de-teclas)
- [Atalhos principais](#atalhos-principais)
- [Configurações e dados](#configurações-e-dados)
- [Limitações conhecidas](#limitações-conhecidas)
- [Documentação de implementação](#documentação-de-implementação)
- [Reportar um problema](#reportar-um-problema)
- [Licença](#licença)

---

## Por que o Yetty

Quem trabalha com servidores, redes ou equipamentos acaba com dezenas de janelas de terminal abertas, senhas anotadas em algum lugar e a mesma conexão digitada todo dia. O Yetty existe para resolver isso:

- **Salve uma vez, conecte sempre.** Guarde servidores e equipamentos em pastas, com usuário e senha (protegida), e abra com dois cliques.
- **Tudo na mesma janela.** Cada conexão vira uma aba, ou fica lado a lado com outras. Mande o mesmo comando para vários servidores de uma vez.
- **Do servidor na nuvem ao equipamento na bancada.** SSH, SFTP, Telnet, RLogin, TCP cru, porta serial, shell local e área de trabalho remota (RDP).
- **Transfira arquivos sem sair do terminal.** Arrastar e soltar no SFTP, Zmodem, Kermit, Xmodem e Ymodem, e um servidor TFTP embutido.
- **Automatize o que se repete.** Scripts, gravação de teclas, biblioteca de comandos prontos e mapa de teclas personalizável.
- **Leve e sem complicação.** Um único arquivo para baixar: no Windows, um `.zip` com o programa; no Linux, um AppImage que roda em praticamente qualquer distribuição. Sem instalador e sem conta.
- **Seu, de graça.** Código aberto sob licença MIT, sem anúncios e sem coleta de dados.

---

## Inspiração

O Yetty nasceu da vontade de ter, de forma livre e gratuita, a praticidade de clientes de terminal consagrados, como o **SecureCRT** e o **QuadCRT**: sessões organizadas em pastas, abas, painéis, transferência de arquivos e muitos detalhes que ajudam no dia a dia de quem administra sistemas.

Não é um clone nem um produto desses programas. É um projeto independente, escrito do zero em Rust, que se inspirou neles na organização e nos atalhos, mantendo o próprio código e o próprio visual. SecureCRT e os demais nomes citados pertencem aos seus respectivos donos, e o Yetty não tem ligação com eles.

---

## Visão geral

Em termos técnicos: cada conexão é uma **sessão** salva em um gerenciador com pastas, e cada sessão aberta vira uma **aba** ou um **painel**. O programa é escrito em Rust, com a interface em [egui](https://github.com/emilk/egui) (via `eframe`) e emulação de terminal com `vt100`.

Pontos de design:

- **Usa as ferramentas do próprio sistema.** SSH e SFTP chamam o `ssh` e o `sftp` do OpenSSH instalado (`ssh.exe` no Windows). O shell local usa ConPTY no Windows e um pty no Linux. RDP abre o `mstsc.exe` no Windows e `xfreerdp`, `remmina` ou `rdesktop` no Linux.
- **Senhas salvas são cifradas**: DPAPI no Windows, ligadas ao usuário que as salvou; ChaCha20-Poly1305 com uma chave local no Linux.
- **Configurações em um único `settings.json`**, sem registro e sem instalador.
- **O mesmo programa e o mesmo visual nas duas plataformas.** O que é específico de um sistema está nas seções Requisitos e Linux.

---

## Download e instalação

Os arquivos ficam na página de [Releases](https://github.com/leoberbert/Yetty/releases). Cada versão traz um arquivo por plataforma, mais o `.sha256` correspondente.

| Plataforma | Arquivo | Como usar |
|---|---|---|
| Windows 10/11 (x64) | `yetty-v<versão>-windows-x64.zip` | Descompacte e execute `yetty.exe`. Não há instalador. |
| Linux x64 | `yetty-v<versão>-linux-x86_64.AppImage` | `chmod +x yetty-*.AppImage` e execute. |

Conferir o download: `sha256sum -c yetty-*.sha256` (Linux) ou `Get-FileHash` (Windows).

**AppImage (Linux)**

- Gerado em um Ubuntu 22.04 (glibc 2.35): roda em distribuições dessa idade ou mais novas. Testado no Ubuntu 26.04 (GNOME) e no CachyOS (KDE Plasma), em Wayland; o programa também abre em X11 (testado fora do AppImage).
- Sem FUSE instalado, execute com `APPIMAGE_EXTRACT_AND_RUN=1 ./yetty-*.AppImage`.
- **Ícone no Wayland:** o ambiente só mostra o ícone do Yetty se existir um `yetty.desktop`. Na primeira execução o AppImage instala `~/.local/share/applications/yetty.desktop` e o ícone em `~/.local/share/icons/hicolor/256x256/apps/yetty.png`, e os atualiza se você mover o arquivo. Na primeira vez o dock pode ainda mostrar o ícone genérico: feche e abra de novo. Para desligar essa instalação, defina `YETTY_NO_INTEGRATION=1`. Para remover, apague esses dois arquivos.

---

## Requisitos

| Item | Detalhe |
|---|---|
| Sistema | Windows 10 ou 11 (x64). Linux: veja a seção [Linux](#linux). |
| Cliente SSH | OpenSSH client do Windows (`ssh.exe`, `sftp.exe`, `ssh-keygen.exe`, `ssh-add.exe`). Vem como recurso opcional do Windows. |
| Git for Windows | Opcional. Recomendado se você usa SFTP com permissões de arquivo completas. |
| Compilação | Toolchain Rust estável com target MSVC (`x86_64-pc-windows-msvc`). |
| Serial | Porta COM física ou virtual. |

---

## Compilar e executar

```bash
# Desenvolvimento
cargo run

# Versão otimizada
cargo build --release
# o executável fica em target/release/yetty.exe
```

---

## Linux

A versão para Linux usa o mesmo código e a mesma interface do Windows. O estado de cada item está em [`PENDENCIAS.md`](PENDENCIAS.md), seções "Versão Linux" e "Laboratório CachyOS".

| Item | Detalhe |
|---|---|
| Sistema | Linux x64 com X11 ou Wayland e OpenGL. Testado no Ubuntu 26.04 (GNOME, Wayland e X11) e no CachyOS (KDE Plasma, Wayland). |
| Compilação | Rust estável, `perl`, `make` e um compilador C (o OpenSSL é compilado junto, então não precisa de `libssl-dev`). Quem usa o AppImage não precisa compilar. |
| Cliente SSH | `openssh-client` (`ssh`, `sftp`, `ssh-keygen`, `ssh-add`), procurados no `PATH`. |
| Diálogos Abrir / Salvar / Pasta | Portal do desktop (`xdg-desktop-portal`, já presente no GNOME e no KDE). |
| Opcionais | `lp` (CUPS) para imprimir; `xfreerdp`, `remmina` ou `rdesktop` para sessões RDP; um emulador de terminal (para Create Identity File e Convert Key). |
| Serial | Dispositivos `/dev/ttyUSB0`, `/dev/ttyS0`... O usuário precisa estar no grupo `dialout`. |

```bash
cargo build --release
# o executável fica em target/release/yetty
```

O AppImage é gerado por `packaging/linux/build-appimage.sh` (precisa do `appimagetool`).

O que muda em relação ao Windows:

- **Configurações** em `~/.config/yetty/settings.json` (ou `$XDG_CONFIG_HOME/yetty`).
- **Senhas salvas** são cifradas com ChaCha20-Poly1305. A chave é um arquivo aleatório, `master.key` (permissão `0600`), na pasta de configurações; a passphrase de configuração entra na derivação da chave (PBKDF2-HMAC-SHA256). Quem copiar só o `settings.json` não lê as senhas. As senhas cifradas de um sistema não abrem no outro: ao levar o `settings.json` do Windows para o Linux, digite as senhas de novo.
- **Shell local** usa `$SHELL` (ou `/bin/sh`) e começa na pasta pessoal.
- **Fontes.** O Windows usa Consolas, Segoe UI e Segoe MDL2 Assets, que não podem ser distribuídos. No Linux o programa traz substitutos livres com as mesmas medidas: Selawik (as medidas do Segoe UI), uma fonte de ícones com os mesmos códigos do Segoe MDL2 Assets e Yetty Mono (a Hack com a largura de célula do Consolas). Para ter exatamente o visual do Windows, copie `consola.ttf`, `segoeui.ttf` e `segmdl2.ttf` para `~/.local/share/fonts` ou para `~/.config/yetty/fonts`: eles têm prioridade.
- **Transparência da janela** (View > Window Transparency) funciona no Linux: a janela inteira, texto incluído, fica translúcida (verificado no KDE Plasma/Wayland).
- **Teclado:** layout ABNT2 verificado byte a byte no KDE (letras, AltGr, teclas mortas, `ç`, Ctrl+letras, F1–F12 e setas).
- **Limites no Linux:** a posição inicial da janela no Wayland é ignorada (o compositor decide); a janela só mostra o ícone do Yetty se houver um `yetty.desktop` (o AppImage cria um sozinho); Ctrl+V e Ctrl+X são colar e recortar e não chegam ao programa.

Para gerar de novo as fontes embutidas: `tools/make-icons-font.py` e `tools/make-mono-font.py`. Licenças em `assets/fonts/`.

---

## Primeiros passos

1. Abra o Yetty. A janela inicial aparece vazia, com o painel **Session Manager** à esquerda.
2. Para uma conexão rápida, use **File > Quick Connect** (`Alt+Q`), escolha o protocolo, informe o host e clique em **Connect**.
3. Para salvar a conexão, marque **Save session** antes de conectar, ou crie a sessão em **File > New Session** (assistente).
4. Para um shell local, use **File > Local Shell** (`Alt+Shift+L`).

<p align="center">
  <img src="docs/images/tela-inicial.png" alt="Janela inicial do Yetty" width="90%">
  <br>
  <em>Janela inicial: menus, barra de ferramentas, painéis laterais e a área de abas vazia.</em>
</p>

---

## Telas

As capturas abaixo são renderizações reais da interface com dados de exemplo.

### Quick Connect

Diálogo de conexão rápida. O protocolo muda os campos exibidos: para SSH2 aparecem firewall, usuário, credenciais e a ordem dos métodos de autenticação (PublicKey, Keyboard Interactive, Password, GSSAPI).

<p align="center">
  <img src="docs/images/quick-connect.png" alt="Diálogo Quick Connect" width="80%">
</p>

- **Protocol**: SSH2, Telnet, Telnet/TLS, Local Shell, RDP, Serial, RLogin ou Raw.
- **Show quick connect on startup**: abre este diálogo sempre que o Yetty iniciar.
- **Save session**: grava a conexão no Session Manager.
- **Open in a tab**: abre a sessão em nova aba em vez de substituir a atual.

### Assistente de nova sessão

Guia passo a passo para criar uma sessão salva. Pode ser desativado com a opção "Do not use this wizard when creating sessions".

<p align="center">
  <img src="docs/images/assistente-nova-sessao.png" alt="Assistente de nova sessão" width="70%">
</p>

### Terminal

Sessão de shell local com cores ANSI, caracteres CJK e símbolos. A barra de status mostra o tipo da sessão, a posição do cursor (linha, coluna), o tamanho (linhas e colunas), o modo de emulação (Xterm) e os indicadores CAP e NUM.

<p align="center">
  <img src="docs/images/terminal-local.png" alt="Sessão de shell local" width="90%">
</p>

### Confirmação de colagem multilinha

Ao colar um texto com mais de uma linha no terminal, o Yetty mostra o conteúdo antes de enviar. Evita executar comandos por engano. Você pode ocultar o diálogo ("Do not show this dialog again") e ajustar o atraso entre linhas ("Override line send delay", padrão 5 ms).

<p align="center">
  <img src="docs/images/colar-multilinha.png" alt="Confirmação de colagem multilinha" width="90%">
</p>

### Senha SSH

Quando o servidor pede senha, o Yetty pergunta antes de conectar. Marque **Save password** para guardá-la cifrada (DPAPI no Windows; chave local no Linux). **Skip** segue sem senha, para tentar outro método de autenticação.

<p align="center">
  <img src="docs/images/senha-ssh.png" alt="Diálogo de senha SSH" width="70%">
</p>

### Verificação de chave de host

Na primeira conexão a um servidor, o Yetty mostra a impressão digital SHA256 da chave do host. Use **View Host Key** para inspecionar, **Accept & Save** para confiar e gravar, ou **Cancel** para abortar.

<p align="center">
  <img src="docs/images/host-key.png" alt="Verificação de chave de host" width="70%">
</p>

### Janela de comandos

Painel inferior para enviar um mesmo texto à sessão ativa, ou ao grupo de abas selecionado. Ideal para repetir comandos sem trocar de aba.

<p align="center">
  <img src="docs/images/janela-comandos.png" alt="Janela de comandos" width="90%">
</p>

### Command Manager

Biblioteca de comandos organizada em pastas, com filtro (`Alt+Y`). Cada item executa uma ação ao ser clicado.

<p align="center">
  <img src="docs/images/gerenciador-comandos.png" alt="Command Manager" width="50%">
</p>

### Propriedades do comando

Define a ação do comando: **Function** (por exemplo, Send String), o texto enviado, rótulo, cor do ícone e descrição. A caixa mostra a lista de sequências especiais aceitas.

<p align="center">
  <img src="docs/images/propriedades-comando.png" alt="Propriedades do comando" width="70%">
</p>

Sequências aceitas em **Send String**:

| Sequência | Efeito |
|---|---|
| `\r` | Retorno de carro (CR) |
| `\n` | Nova linha (LF) |
| `\t` | Tabulação |
| `\e` | Escape |
| `\b` | Backspace |
| `\\` | Barra invertida literal |
| `\p` | Pausa de um segundo |

### Layout lado a lado

Modo **Tile Vertically**: duas sessões ocupam o espaço lado a lado, e cada uma continua independente.

<p align="center">
  <img src="docs/images/layout-lado-a-lado.png" alt="Dois painéis lado a lado" width="90%">
</p>

### Grupos de abas

Abas podem ser agrupadas. Um grupo recebe comandos da janela de comandos ao mesmo tempo, o que permite operar vários servidores com um único envio.

<p align="center">
  <img src="docs/images/grupos-abas.png" alt="Grupos de abas" width="90%">
</p>

### Opções globais

Árvore de configurações com páginas para caminhos, sessão automática, credenciais, terminal, aparência, paleta ANSI, barra de status, abas, mosaico, impressão, navegador, servidor TFTP, firewall, SSH2, chaves do agente e chaves de host.

<p align="center">
  <img src="docs/images/opcoes-globais.png" alt="Opções globais" width="90%">
</p>

### Mapa de teclas

Editor visual de mapeamento. Você escolhe a tecla (com Ctrl, Alt e Shift) e a função, ou envia uma string. Permite salvar e carregar mapas.

<p align="center">
  <img src="docs/images/mapa-teclas.png" alt="Editor de mapa de teclas" width="80%">
</p>

---

## Funcionalidades

### Protocolos e conexões

| Protocolo | Como funciona | Porta padrão |
|---|---|---|
| **SSH2** | Executa o `ssh` (`ssh.exe` no Windows) do OpenSSH em uma aba. Suporta autenticação por chave pública, teclado interativo, senha e GSSAPI. | 22 |
| **SFTP** | Sessão SFTP via `sftp.exe`. Ativada pela opção "SFTP session" dentro de SSH2. | 22 |
| **Telnet** | Conexão TCP com negociação Telnet. | 23 |
| **Telnet/TLS** | Telnet sobre TLS, com SChannel (`native-tls`). | 992 |
| **RLogin** | Protocolo RLogin sobre TCP. | 513 |
| **Raw** | TCP puro, sem negociação. | 23 |
| **Serial** | Porta COM (Windows) ou `/dev/ttyUSB0`, `/dev/ttyS0`... (Linux, grupo `dialout`) com baud, bits, paridade, stop bits e controle de fluxo. Padrão: 38400, 8N1, XON/XOFF. | — |
| **Local Shell** | Windows: PowerShell, cmd ou outro, via ConPTY. Linux: `$SHELL`, em um pty. | — |
| **RDP** | Inicia um cliente de Remote Desktop em janela própria: `mstsc.exe` no Windows; `xfreerdp`, `remmina` ou `rdesktop` no Linux. | 3389 |

Na linha de comando, o protocolo pode ser passado diretamente: `ssh`, `telnet`, `rlogin`, `raw` (ou `tcp`) e `local` (ou `shell`, `cmd`).

### Gerenciador de sessões

Painel **Session Manager** à esquerda (`Alt+M`).

- Sessões organizadas em **pastas**, com criação, renomeação, exclusão e duplicação.
- **Arrastar e soltar** para mover sessões entre pastas.
- **Filtro** por nome ou host (`Alt+I`).
- Duplo clique conecta. `F2` renomeia, `Del` exclui, `Ctrl+X`, `Ctrl+C` e `Ctrl+V` recortam, copiam e colam.
- **Active Sessions** (`Alt+Z`) lista as sessões abertas e permite trocar de uma para outra.
- **Reconnect all** (`Alt+A`) reconecta todas as sessões abertas.

Cada sessão guarda protocolo, host, porta, usuário, método de autenticação, esquema de cores, fonte, tamanho do terminal, scrollback e opções de log.

### Abas, painéis e layout

- Abas com botão de fechar, arrastar para reordenar e clique do meio para fechar.
- `Ctrl+Tab` e `Ctrl+Shift+Tab` alternam entre abas; `Ctrl+F4` fecha a aba atual.
- **Tile Vertically** e **Tile Horizontally** dispõem as sessões lado a lado.
- **Cascade** sobrepõe as janelas em escada.
- **Full screen** com `Alt+Enter`.
- Zoom da fonte do terminal com `Ctrl+=`, `Ctrl+-` e `Ctrl+0`.
- Transparência da janela e opção "sempre no topo" em **Window**.
- Painéis laterais podem ser fixados ou ocultados automaticamente.

### Emulação de terminal e aparência

- Emulação **Xterm** com cores ANSI, UTF-8 e caracteres de largura dupla.
- Tamanho padrão de 80 colunas por 24 linhas; scrollback padrão de 500 linhas.
- Fonte do terminal padrão: Consolas 13. Fonte da interface: Segoe UI 12.
- **Temas da interface**: System, Native, Carbon, VS Blue, VS Dark, VS White e Office White.
- **Esquemas de cor** do terminal: 23 opções, de cores clássicas a temas inspirados em editores.
- Paleta ANSI editável em **Global Options > ANSI Color**.
- Realce de palavras-chave: busca com `Alt+U`.
- **Color Scheme Overrides Ansi Color** (opção de sessão, ligada como no `Default.ini` do SecureCRT): com um esquema em uso, as 16 cores ANSI são as da paleta do esquema (por exemplo Solarized Dark), e não as de Global Options > ANSI Color. O System Color Scheme usa a paleta Dark Pastel.
- **Jump Scroll** e **Minimize Drawing While Jump Scrolling** (opções de sessão, ligadas como no original; Session Options > Emulation): a saída em massa é desenhada a cada 120 ms (250 ms com Minimize Drawing); um trecho pequeno com nada retido, como o eco de uma tecla, aparece na hora (digitar não sofre atraso).
- **Select an alternate keyboard emulation** (Session Options > Emulation): Default, Linux, SCOANSI, VT100, VT220, VT320, Wyse50, Wyse60, Xterm, Xterm R6, Xterm 216+, VShell, TVI910, TVI925 e TN3270, com os códigos de F1 a F12 dos arquivos `Keymaps` do SecureCRT. A opção Custom não existe.
- **Mouse**: programas que pedem o mouse (vim, tmux) o recebem no padrão xterm; `Shift` mantém a seleção local (**Shift forces local mouse**). **Position cursor on click** move o cursor da linha de comando com um clique.
- **Barra de rolagem horizontal** para terminais de tamanho fixo mais largos que a janela; `Shift`+roda rola de lado.
- **Barras de botões** com nome (menu de contexto do botão ➕) escolhidas por sessão, e **posição inicial** da janela.
- **Mnemônicos de menu** do SecureCRT: `Alt`+letra abre o menu e as letras sublinhadas escolhem os itens, abrem submenus e marcam opções (menus View, Options, Themes e Window).

### Transferência de arquivos

Menu **Transfer**.

- **Xmodem, Xmodem-1K e Ymodem**, em envio e recebimento. Pode ser enviado em modo **ASCII** ou **Binary**.
- **Servidor TFTP** embutido, na porta UDP 69, com pasta escolhida pelo usuário. Útil para equipamentos que fazem boot por rede.
- **SFTP**: navegar no servidor e transferir por arrastar e soltar de arquivos do Windows para a sessão.

- **Kermit** e **Zmodem**, em envio e recebimento. No Zmodem há a **Zmodem Upload List** e o **Start Zmodem Upload**, que executa antes o comando de upload da sessão (padrão `rz`). Um pedido `sz` do servidor inicia o download sozinho (desligável em Session Options). Testados em loopback e contra `sz`/`rz` (lrzsz 0.12.21) e `gkermit` 2.01 reais, com arquivo de 200 000 bytes.

### Scripts e gravação

Menu **Script**.

- **Run** executa um script em arquivo de texto.
- **Edit** abre o script no editor.
- **Recording** grava as teclas digitadas e gera um script equivalente.

A linguagem é deliberadamente simples, com comandos como `connect`, `send`, `sendln`, `waitfor` e `sleep`. Não há suporte a VBScript nem a Python.

### Janela de comandos e Command Manager

- **Command Window** (`Alt+G`): envia texto à sessão ativa ou ao grupo selecionado.
- **Command Manager** (`Alt+J`): biblioteca de comandos em pastas, com cor, rótulo e descrição.
- Atalhos `Ctrl+Shift+I`, `Ctrl+Shift+W`, `Ctrl+Shift+T`, `Ctrl+Shift+S` e `Ctrl+Shift+B` acionam ações da janela de comandos.

### Chaves SSH e credenciais

Menu **Tools**.

- **Create Public Key**: abre `ssh-keygen -t ed25519` em uma aba.
- **Convert Private Key**: converte para o formato OpenSSH (`ssh-keygen -p -o`).
- **Public-Key Assistant**: mostra a chave pública e a impressão digital de um arquivo de chave.
- **Manage Agent Keys**: lista e gerencia chaves do `ssh-agent` (`ssh-add -l`).
- **Manage Saved Credentials**: lista e remove senhas salvas.

- **Export Public Key from Certificate**: extrai a chave pública (RSA, ECDSA, Ed25519) de um certificado PEM, DER ou PFX/P12 para uma linha OpenSSH.
- **Change Configuration Passphrase**: protege as senhas salvas com uma passphrase pedida ao iniciar o Yetty. Se for esquecida, as senhas salvas se perdem.

Senhas salvas são cifradas (DPAPI no Windows, lidas só pelo mesmo usuário; ChaCha20-Poly1305 no Linux) e não passam de um sistema para o outro. Com passphrase, ela entra também na cifragem.

Quando a chave de identidade escolhida é recusada pelo servidor, abre a janela **Public Key Authentication Failed**.

### Log, impressão e snapshots

- **Log Session** grava a saída da sessão em arquivo. **Raw Log** grava o fluxo bruto, com as sequências de controle. **Trace Options** registra o tráfego de protocolo para diagnóstico.
- **Printing** envia o conteúdo do terminal à impressora.
- **Snapshots** salvam o conjunto de sessões abertas para reabrir depois.
- **Multi-line paste confirm**: ver a seção de colagem multilinha acima.

### Mapa de teclas

**Tools > Keymap Editor** permite:

- Associar qualquer tecla (com modificadores) a uma função do Yetty ou a uma string enviada ao servidor.
- Salvar e carregar mapas.
- Ver visualmente o teclado para escolher a tecla.

---

## Atalhos principais

| Atalho | Ação |
|---|---|
| `Alt+Q` | Quick Connect |
| `Alt+C` | Conectar a sessão selecionada |
| `Alt+B` | Conectar em nova aba |
| `Alt+Shift+L` | Local Shell |
| `Alt+R` | Foco na caixa de host (conexão rápida) |
| `Alt+U` | Foco na caixa de palavra-chave |
| `Alt+M` | Session Manager |
| `Alt+Z` | Active Sessions |
| `Alt+J` | Command Manager |
| `Alt+G` | Command Window |
| `Alt+Y` | Filtro de comandos |
| `Alt+I` | Filtro de sessões |
| `Alt+P` | Sessão SFTP |
| `Alt+A` | Reconectar todas |
| `Alt+Enter` | Tela cheia |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Zoom in / out / padrão |
| `Ctrl+Ins` | Copiar |
| `Shift+Ins` | Colar |
| `Ctrl+Shift+A` | Selecionar tudo |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Próxima / anterior aba |
| `Ctrl+F4` | Fechar aba |
| `F2` / `Del` | Renomear / excluir item no gerenciador |

---

## Configurações e dados

- **Configurações**: `%APPDATA%\Yetty\settings.json` no Windows e `~/.config/yetty/settings.json` no Linux. Sessões, comandos, opções e mapa de teclas ficam nesse arquivo.
- **Migração**: se existir a pasta `%APPDATA%\NexTerm` de uma versão anterior e ainda não existir `%APPDATA%\Yetty`, ela é renomeada automaticamente na primeira execução.
- **Export/Import Settings** (menu **Tools**) permite levar a configuração para outro computador.
- Se `APPDATA` não estiver definido, o Yetty usa a pasta temporária do sistema.

---

## Limitações conhecidas

Estes itens **não estão implementados**:

- Kermit e Zmodem foram testados em loopback e contra `gkermit`, `sz` e `rz` no WSL, não contra outros servidores (por exemplo C-Kermit).
- SSH1.
- TAPI.
- Scripts em VBScript e Python.
- Grupos de abas com funcionalidade parcial.
- Teclado alternativo "Custom" e TN3270; a lista de sessões do menu Window não tem mnemônico.

Telnet/TLS depende do SChannel no Windows (e do OpenSSL embutido no Linux) e pode não aceitar todos os certificados de servidores antigos.

No Linux, ainda sem teste em hardware: porta serial real, impressão (CUPS) e RDP. A versão do Windows depende do checklist em [`docs/CHECKLIST-WINDOWS.md`](docs/CHECKLIST-WINDOWS.md) a cada release.

---

## Documentação de implementação

O arquivo [`MECHANICS.md`](MECHANICS.md) registra, em 36 rodadas, como cada recurso foi implementado e comparado com o comportamento do programa original que serviu de referência. Os itens não implementados estão descritos lá com o motivo.

---

## Reportar um problema

Abra uma issue em <https://github.com/leoberbert/Yetty/issues/new/choose> e preencha o formulário de bug: ele pede plataforma (Windows ou Linux), versão, como foi instalado (zip, AppImage ou compilado), sistema (no Linux, distribuição, ambiente gráfico e Wayland ou X11), os passos e se acontece na outra plataforma. Roteiros de teste manual: [`docs/ROTEIRO-REGRESSAO-LINUX.md`](docs/ROTEIRO-REGRESSAO-LINUX.md) e [`docs/CHECKLIST-WINDOWS.md`](docs/CHECKLIST-WINDOWS.md).

---

## Licença

Distribuído sob a licença MIT. Veja [`LICENSE`](LICENSE).

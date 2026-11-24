# Roteiro de regressão manual — Yetty no Linux

Para o que não dá para automatizar (teclado, mouse, janelas do sistema, hardware).
Marque cada item: OK / falhou (anote o que viu). Use o AppImage (`dist/yetty-v*.AppImage`) ou `cargo run --release`.
Para isolar suas configurações: `XDG_CONFIG_HOME=/tmp/yt-teste ./yetty`.

## 1. Teclado (L-11) — o mais importante
Num teclado ABNT2, abra um Local Shell e rode `cat -v` (mostra o que chega):
1. Digite `/ ? ; : ~ ^ ` ' " \ | { } [ ] @ # $ % & * ( )` e `ç Ç á é ã õ` (acentos mortos: `~` + `a`, `'` + `e`). Deve chegar igual ao digitado.
2. AltGr + teclas (`€`, `¢`, `¹²³`) — deve digitar o caractere, não acionar atalho.
3. Ctrl+C, Ctrl+D, Ctrl+Z, Ctrl+L, Ctrl+A/E (readline), Ctrl+setas.
4. F1–F12, Home/End, PgUp/PgDn, Insert/Delete, setas; no `vim` e no `htop`.
5. Alt+F, Alt+E, Alt+V, Alt+O, Alt+T, Alt+S, Alt+W, Alt+H abrem os menus (letras sublinhadas); dentro do menu, a letra escolhe o item.
6. Alt+R (barra Connect), Alt+U (Keyword), Alt+G (Command Window), Alt+I (filtro do Session Manager).
7. Shift+Insert / Ctrl+Shift+V colam; Ctrl+Shift+C copia (se configurado).

## 2. Mouse, seleção e área de transferência (L-23)
1. Selecionar texto com o mouse, copiar (Edit > Copy) e colar noutra aplicação; colar de outra aplicação (inclusive multilinha: aparece a janela "Paste multi-line").
2. Duplo clique seleciona palavra; triplo, linha.
3. Roda do mouse rola o histórico; no `vim` com `:set mouse=a` e no `tmux` (`set -g mouse on`) clique e roda vão ao programa; **Shift+clique** força seleção local.
4. Arrastar um arquivo do gerenciador de arquivos para uma sessão SFTP (upload).
5. Botão direito: menu de contexto do terminal, das abas, da barra de botões.

## 3. Janela, temas e transparência
1. View > Themes: alternar System / Native / Carbon / VS Blue / VS Dark / VS White / Office White com abas abertas e um diálogo aberto; nada pode ficar com cor errada ou ilegível.
2. **View > Window Transparency** (L-19): ligado, a janela deve ficar translúcida (texto também), desligado volta opaca. Testar no Wayland e, se puder, numa sessão X11. Comparar abas, menus abertos e barra de status.
3. Redimensionar, maximizar, minimizar, tela cheia; ícone na barra/dock é o do Yetty (WM_CLASS `yetty`).
4. Session Options > Window > "Initial position": no Wayland é ignorada (esperado); no X11 a janela deve ir para x, y.
5. Zoom (Ctrl+= / Ctrl+- / Ctrl+0) e fonte do terminal.
6. Tile (Window > Tile Vertically/Horizontally), Cascade, e grupos de abas arrastando uma aba.

## 4. Conexões
1. **SSH com senha** e com keyboard-interactive: aparece a janela de senha (sem eco), erro de senha errada mostra "Password Authentication Failed".
2. **SSH, host novo**: janela "New Host Key" → "Accept & Save" salva; reconectar não pergunta; trocar a chave do servidor → aviso de chave alterada.
3. SSH com chave com passphrase; com ssh-agent; Agent Forwarding; X11 forwarding; port forwarding (Session Options > Port Forwarding).
4. SFTP: `ls`, `get`, `put`; arrastar arquivo; pasta inicial.
5. Telnet, Telnet/TLS, Rlogin, Raw (já testados contra servidor de teste; confirme contra um servidor seu).
6. **Serial** (L-13): com um adaptador USB (`/dev/ttyUSB0`, usuário no grupo `dialout`): lista de portas, abrir, **digitar** e ver eco, trocar velocidade/paridade/controle de fluxo.
7. **RDP** (L-14): precisa de `xfreerdp`/`remmina`; sem nenhum deve aparecer "no Remote Desktop client found".
8. Sessão salva: criar, editar, duplicar, mover para pasta, excluir; Quick Connect; "Connect in Tab"; reabrir ao iniciar (Auto Session).

## 5. Transferências
1. Zmodem: num servidor com `lrzsz`, `sz arquivo` (download automático) e Transfer > Zmodem Upload List / `rz` (upload).
2. Kermit: Transfer > Kermit Send/Receive contra `gkermit`/`kermit`.
3. Arquivos grandes (>10 MB) e nomes com acento/espaço.

## 6. Diálogos do sistema (L-10)
1. Todo botão "Browse…" (identity file, known_hosts, pasta SFTP, script, log): abre o seletor do GNOME (portal) e devolve o caminho. Sem portal instalado, deve só não escolher nada.
2. Salvar log de sessão, Save Screen, Export/Import de configurações.

## 7. Ferramentas e integração
1. Tools > Create Public Key (abre `ssh-keygen` numa aba), Manage Agent Keys, Convert/Export Public Key from Certificate.
2. **Create Identity File** (L-16) abre um emulador de terminal do sistema e espera Enter no fim.
3. Session Manager > **Create Shortcut** (L-17): o `.desktop` aparece na área de trabalho, tem permissão de execução e abre a sessão certa (incluindo nomes com aspas/`$`).
4. **Imprimir** (L-15): Print Selection / Print Screen com uma impressora CUPS; sem `lp` aparece mensagem na barra de status.
5. Change Configuration Passphrase: definir, fechar, reabrir (pede a passphrase; errada é recusada), remover.
6. Senhas salvas: salvar uma, fechar, reabrir e conectar sem digitar.
7. Command Window / Command Manager: enviar a "Active session" e a "All Sessions"; Undo (botão direito).
8. Scripts (Script > Run/Start Recording), Keymap Editor (Ctrl+F5 → Send String), Button Bar.

## 8. Barra de status e teclado de bloqueio
1. CAP e NUM acendem/apagam com Caps Lock e Num Lock (L-12); em teclado sem LED mostram apagado.
2. Linhas/colunas, tipo de terminal e protocolo corretos.

## 9. Instalação / empacotamento (L-20)
1. Rodar o AppImage numa **outra distribuição** (Debian/Fedora estáveis, glibc mais velha) e sem FUSE (`--appimage-extract-and-run`).
2. Ícone e nome corretos depois de integrar (`.desktop`).
3. No GitHub: Actions > Release (versão igual à do Cargo.toml) gera zip do Windows **e** o AppImage; Actions > CI roda os jobs Linux.

## 10. Windows (não foi executado aqui)
Só compilou (`cargo check --target x86_64-pc-windows-msvc`). Repetir os itens 1–8 no Windows, em especial: senhas salvas (DPAPI), transparência (`LWA_ALPHA`), ícones Segoe MDL2, AltGr, RDP (`mstsc`), Create Shortcut (`.lnk`).

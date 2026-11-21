# Controle de pendências — Yetty vs SecureCRT

Origem: regressão da Round 36 (`MECHANICS.md`). Atualizar este arquivo a cada rodada.
Legenda: **Fechado** = implementado e verificado como indicado; **Estimado** = comportamento inferido, sem a referência do original à mão.

| ID | Item | Status | Observação |
|---|---|---|---|
| REG-01 | Command Window: destino "All Sessions" | Fechado | Não era regressão (Round 35); verificado na Round 36 |
| DIV-07..13 | Tile, Global Options, menu de abas, ícones, RDP, Telnet/TLS, paleta | Fechado | Rounds 34–36 |
| BUG-03 | Janelas de host key / senha sem eco no terminal | Fechado | Round 36 |
| P-01 | Command Window: Undo (menu de contexto) | Fechado | Usa o histórico do `TextEdit`; compila, **não testado na janela** |
| P-02 | Janela "Public Key Authentication Failed" | Fechado (texto estimado) | Com identity file escolhido o ssh roda com `-v`; as linhas `debug1:` ficam fora da tela. Teste unitário em `term.rs`. O texto da janela é estimado |
| P-03 | "Color scheme overrides ANSI color" | Fechado (semântica estimada) | Opção em Global Options > ANSI Color (padrão ligado, como `Global.ini`=1): com esquema de cores em uso, ANSI preto/branco usam fundo/texto do esquema. A paleta "Standard" continua a medida na Round 35 |
| P-04 | Export Public Key from Certificate | Fechado | Tools; PEM, DER e PFX/P12 (sem senha); RSA, ECDSA (P-256/384/521) e Ed25519. Chaves conferidas com `ssh-keygen -y` (`certkey.rs`) |
| P-05 | Kermit (Send / Receive) | Fechado | `kermit.rs`, checksum tipo 1, prefixo de controle e de 8 bits. Testado em loopback, inclusive pacote danificado; **não testado contra um host real** |
| P-06 | Zmodem: Upload List, Start Zmodem Upload, download automático | Fechado | `zmodem.rs`, CRC-16 (aceita CRC-32). Testado em loopback com todos os bytes e com erro de CRC; **não testado contra `rz`/`sz` reais**. Comando de upload (padrão `rz`) e download automático em Session Options > X/Y/Zmodem |
| P-07 | Change Configuration Passphrase | Fechado | Senhas salvas passam a usar a passphrase como entropia do DPAPI (`dpapi2:`); janela no início pede a passphrase. Teste de troca/remoção em `config.rs`; janela de pedido não exercitada com o programa aberto |
| P-08 | Mnemônicos de menu | Fechado | Capções e letras vindas de `Menus/default.mnu` do original (`mnemonics.rs`). Verificado na janela: Alt+T abre Transfer, letras sublinhadas, `Z` abre Zmodem Upload List. Submenus e itens do menu Window (botões de opção) ainda sem mnemônico |
| P-09 | Session list sort order | Fechado | O original tem uma caixa "Sort alphabetically" (texto lido em `SecureCRT.exe`), não uma lista; trocado |
| P-10a | Jump scroll | Fechado (semântica estimada) | Saída rápida é desenhada a cada 120 ms |
| P-10b | Minimize drawing | Fechado (semântica estimada) | Redesenho no máximo a cada 50 ms |
| P-10c | Shift forces local mouse | Fechado | Exigiu implementar o relato de mouse do xterm (X10/normal/botão/qualquer movimento, SGR e padrão) — antes nenhum clique ia ao programa remoto. Opção "Send mouse events…" (ligada) controla o relato. **Não testado contra vim/tmux** |
| P-10d | Position cursor on click | Fechado | Clique na linha do cursor envia setas até a coluna |
| P-10e | Alternate keyboard | Fechado (lista estimada) | None / Linux / Xterm R5-VT220: códigos de F1–F5. A lista do original não foi vista |
| P-10f | Horizontal scroll bar | Fechado | Session Options > Window e View; vale com "Disable resize" e terminal mais largo que a janela; Shift+roda rola de lado |
| P-10g | Button bar choice | Fechado | Barras nomeadas: menu de contexto do "➕" cria/exclui; Session Options > Window escolhe |
| P-10h | Initial position | Fechado | A janela do Yetty vai para x, y quando a primeira sessão abre |
| X-01 | SSH1, TAPI | Descartado | Decisão do usuário (Round 35) |
| X-02 | Open SecureFX | Descartado | Produto separado; cinza também no original |

# Checklist curto — Yetty no Windows

O código do Windows só foi compilado (`cargo check --target x86_64-pc-windows-msvc`), nunca executado nesta rodada.
Rode com o `.zip` da release ou `cargo run --release`. Marque OK / falhou (anote o que viu). ~15 minutos.

## Mudanças recentes que afetam o Windows (testar primeiro)
1. **Digitar rápido:** abra um Local Shell (cmd/PowerShell) e uma sessão SSH; digite rápido e segure uma tecla. O eco deve acompanhar sem saltos. Compare com Session Options > Terminal > Jump scroll desligado: não deve haver diferença perceptível.
2. **Saída grande:** `dir /s C:\Windows` ou `type arquivo_grande.txt`: rola sem travar e termina na última linha, com o prompt certo.
3. **Tema VS Blue** (View > Themes): barra de status azulada (não cinza); VS Dark continua `333333`; temas claros sem texto ilegível.
4. **Zmodem:** receber um arquivo com `sz` e enviar com `rz` (ou Transfer > Zmodem Upload List) em um servidor Linux; o arquivo chega igual.

## Específicos do Windows (não podem ter quebrado)
5. **Senhas salvas (DPAPI):** salvar senha numa sessão SSH, fechar, reabrir e conectar sem digitar; Tools > Change Configuration Passphrase: definir, reabrir (pede), remover.
6. **Transparência:** View > Window Transparency liga/desliga (janela translúcida e volta opaca).
7. **AltGr / ABNT2:** digitar `/ ? \ | { } @ ç` e acentos mortos (`~a`, `'e`) num Local Shell com `type con` ou no PowerShell.
8. **Ícones e fontes:** ícones da barra (Segoe MDL2), menus e diálogos com a fonte Segoe UI, terminal em Consolas.
9. **Create Shortcut** (Session Manager): cria o `.lnk` na Área de Trabalho e abre a sessão certa.
10. **RDP:** uma sessão Remote Desktop abre o `mstsc`.
11. **Pasta de configurações:** `%APPDATA%\Yetty\settings.json` é criado e relido.

## Conexões básicas
12. SSH (senha e chave), SFTP, Telnet e Local Shell abrem, mostram cores e aceitam teclado.
13. Host novo: janela "New Host Key" → Accept & Save; reconectar não pergunta.

## Se algo falhar
Anote: versão (Help > About), sessão/protocolo, o que apareceu e se o Linux faz diferente.

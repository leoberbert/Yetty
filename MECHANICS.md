# Yetty — mechanics sheet

Values read from the original program's code (Deep mode, Ghidra pseudo-code and raw data).
Our code is written from this sheet; nothing decompiled is copied.

## Terminal defaults — session profile loader `0x1402625f0`

| Setting | Value | Where |
|---|---|---|
| Emulation | Xterm | string set just before `"Emulation"` key (0x1402625f0) |
| Rows | 24 (0x18) | `"Rows"` default |
| Columns | 80 (0x50) | `"Cols"` default |
| Scrollback lines | 500 | `"Scrollback"` default |
| ANSI colour | on | `"ANSI Color"` default 1 |
| Line wrap | on | `"Line Wrap"` default 1 |
| Cursor key mode (application) | off | `"Cursor Key Mode"` default 0 — switched by the host (`"Enable Cursor Key Mode Switching"` = 1) |
| Blinking cursor | on | `"Blinking Cursor"` default 1 |
| Cursor style | 0 = block | `"Cursor Style"` default 0 |
| Output encoding | UTF-8 | `"Output Transformer Name"` default |
| Resize mode | 0 = rows/cols follow the window | `"Resize Mode"` default 0 |
| Colour scheme | "System Color Scheme" (system window text on window background) | `0x1402610d0` |
| Alternate screen added to scrollback | off | default 0 |

## Default 16-colour ANSI palette — data `0x140435950` (RGB bytes)

0 000000 · 1 a00000 · 2 00a000 · 3 a0a000 · 4 0000a0 · 5 a000a0 · 6 00a0a0 · 7 c0c0c0 ·
8 808080 · 9 ff0000 · 10 00ff00 · 11 ffff00 · 12 0000ff · 13 ff00ff · 14 00ffff · 15 ffffff

## Colour schemes — registered in `0x14023b640`, tables from `0x140437ab0` (64 bytes each; text = slot 0, background = slot 8)

| Scheme | Text | Background |
|---|---|---|
| Birds of Paradise | e0dbb7 | 2a1f1d |
| Black / Floral White | 000000 | fffbf0 |
| Chalkboard | d9e6f2 | 29262f |
| Ciapre | aea47a | 191c27 |
| Dark Pastel | ffffff | 000000 |
| Desert | ffffff | 333333 |
| Espresso | ffffff | 323232 |
| Floral White / Dark Cyan | fffbf0 | 008080 |
| Idletoes | ffffff | 323232 |
| Invisibone | a0a0a0 | 232323 |
| Monochrome | 000000 | ffffff |
| Novel | 3b2322 | dfdbc3 |
| Solarized Dark | 839496 | 002b36 |
| Solarized Darcula | d2d8d9 | 3d3f41 |
| Solarized Light | 657b83 | fdf6e3 |
| Tomorrow | 4d4d4c | fafafa |
| Traditional | 00ff00 | 000000 |
| White / Black | ffffff | 000000 |
| White / Blue | ffffff | 0000ff |
| Yellow / Black | ffff00 | 000000 |
| Zenburn | dcdccc | 3f3f3f |

"Black / Cyan" exists too but its table uses flag bytes we could not decode, so it is left out.

## Serial defaults — `0x14021e7b0`

Baud 38400 (0x9600) · parity none (0) · stop bits 1 (0 = ONESTOPBIT) · data bits 8 · XON/XOFF flow on · DSR/CTS flow off.

## Host settings — `0x14028c040`

Hostname, Username and Firewall ("None") are per-session; ports are not set here (they live in the
protocol modules). **Estimated:** SSH 22, Telnet 23, RLogin 513, raw TCP 23 (well-known ports).

## Protocols offered (strings referenced by the session code)

SSH2, SSH1 (enable file), Telnet, Telnet/TLS, RLogin, Serial, TAPI, Raw, Local Shell, RDP.
This version builds: SSH2 (through the system's OpenSSH client), SFTP tab, Telnet, RLogin, Raw TCP,
Serial and Local Shell. Left out: SSH1, Telnet/TLS, TAPI, RDP.

## Menus and shortcuts — `Menus/default.mnu`

Alt+C connect, Alt+B connect in tab, Alt+Shift+L local shell, Alt+Q quick connect,
Alt+M session manager, Alt+G command window, Ctrl+= / Ctrl+- / Ctrl+0 zoom.
Edit: Copy, Paste, Select All, Find, Clear Scrollback / Screen / both, Send Break, Reset.
View: Toolbar, Session Tabs, Command Window, Connect Bar, Status Bar, Button Bar, Session Manager.
Options: Session Options, Global Options, Keyword Highlighting.

## Estimated (looked for, not found in the code)

- Font size 14 px (the original uses a Windows font picker; default face not read).
- Keyword list starts with two sample words ("error", "warning").
- Button bar starts with two sample buttons.

## Session options implemented (round 2, matches the original's Session Options tree)

Connection (protocol, hostname, port, firewall, username/credentials, authentication order) · Logon Actions
(automate logon expect/send, initial CR, logon script, remote command) · SSH2 (SFTP session, advanced, host key,
port forwarding -L/-R/-D, X11/agent) · Terminal (emulation, modes, emacs/meta, mapped keys, advanced) ·
Appearance (colour scheme, font size, cursor, encoding, window) · Keyword highlighting (style/colour) ·
Log File (name variables %H %S %Y %M %D %h %m %s %t, append/overwrite, timestamps, start on connect — decompiled
keys "Log Filename V2", "Start Log Upon Connect" at 0x140357498 / 0x140357620) · Printing · X/Y/Zmodem folders.
Quick Connect fields (Protocol, SFTP session, Hostname, Port, Firewall, Username/Credentials, Authentication list
PublicKey / Keyboard Interactive / Password / GSSAPI, Show quick connect on startup, Save session, Open in a tab)
copied from the person's screenshots of the original (attachments/picture-1..3.png).

Not built: Xmodem/Ymodem/Zmodem/Kermit, TFTP server, VBScript/Python scripting (a simple text script is offered
instead), RDP, SSH1, Telnet/TLS, TAPI, credential store, key conversion, cascade windows, window transparency.

## Authentication properties (round 3, from the person's screenshot attachments/picture-4.png)

Authentication list (PublicKey, Keyboard Interactive, Password, GSSAPI): check = enabled, click = select,
arrows = order, gear / double-click = properties of the selected method. PublicKey: global or session identity
file (Browse, keys found in .ssh, ssh-agent on/off, Create Identity File). Password / Keyboard Interactive: optional
saved password sent at the "password:" prompt. GSSAPI: delegate credentials. SSH2 Advanced: key exchange, cipher
and MAC order; Host Key: host key algorithms and verification mode. Order and checks become ssh options
(PreferredAuthentications, KexAlgorithms, Ciphers, MACs, HostKeyAlgorithms, -i, IdentityAgent).

## Round 4

Tabs: close button on each tab with its own click area, middle-click closes, right-click menu (Clone Session,
Connect SFTP Tab, Lock Session, Reconnect, Disconnect, Rename Tab, Session Options, Close, Close All But This,
Close Tabs to the Right, Close All Tabs). Session Options of a connected tab are written back to the saved session.
The key chosen in PublicKey Properties is stored in the session and in the global setting, and settings are saved
when a dialog is closed. Menus now complete: Xmodem / Xmodem-1K / Ymodem send and receive (loopback-tested),
Raw Log Session, Trace Options (ssh -v), Auto Print, Cascade, Saved Credentials, key conversion/export, Tools import.
Not built: Kermit, Zmodem, TFTP server, RDP, window transparency, configuration passphrase.

## Round 6 — Session Options pages reviewed

Modes: auto wrap, origin, insert, new line (LF also returns the cursor), local echo, application cursor/keypad,
cursor-key mode switching. Emacs: Alt as Meta (ESC prefix or 8-bit). Mapped Keys: Capture a key, send text.
Terminal Advanced: Backspace / Delete / Enter keys, audible and visual bell. Emulation: more terminal types and an
answerback text sent on ENQ. Window: tab colour. Keyword Highlighting Advanced: style, colour, match case, whole line.
Host Key: known-hosts file. Remote/X11: X11 forwarding, trusted (-Y), display, agent forwarding.

## Round 7 — authentication dialogs as in the original (attachments/picture-25..29.png)

Quick Connect: Authentication list with full-width blue selection, up/down arrows and a "Properties..." button.
Password Properties: "Store password" + "Confirm password", "(Password will be encrypted.)" — the password is encrypted
with the Windows user key (DPAPI) before it is written to the settings file; "Change Remote Password > Change Password"
opens a tab that runs passwd on the server. Keyboard Interactive shares the password box. GSSAPI Properties: Method
(Auto Detect / SSPI / Kerberos), Delegation (Full / None), Advanced >> (server principal name); OK and Cancel (Cancel
puts the previous values back). The session tree also shows Default_LocalShell and Default_Serial (templates).

## Round 9 — menus as in the original (attachments/picture-36..43), TFTP, checks, file dialogs

Menus now enable and disable like the original when no session is open: only the commands that need a session are grey
(Reconnect, Disconnect, Clone, Lock, Send/Receive, Find, Page Setup, Pause Logging, Keyword Highlighting...). Log Session,
Raw Log Session and Trace Options are check boxes that can be turned on before a session exists and apply to the next one.
"Start TFTP Server" works at any time: a UDP server on port 69 (RFC 1350, read and write, files only inside the chosen
folder, loopback-tested). Side tabs Session Manager / Command Manager / Active Sessions follow the View check boxes.
Field checks (validate_session): hostname, port 1-65535, session and folder names (characters, duplicates), serial port,
saved credential, identity file, remote command, logon script, port forwards, mapped keys, anti-idle string, word
delimiters, log file name. The Windows Open / Save / folder dialogs are called directly (no helper process), so they open
at once. Page title bars follow the theme. Not built: Change Configuration Passphrase, Kermit, Zmodem.

## Round 10 — Session Manager drag and drop, menus that follow the pointer

Session Manager: press and drag a session or a folder and drop it on a folder (or on the "Sessions" root, or on another
session to land in that session's folder). Folders are moved with everything inside; a folder cannot go into itself or into
one of its own sub-folders, and a name already used in the target folder is refused with a message. Folders are listed
before sessions. Menu bar: once one menu is open, moving the pointer over another top-level menu opens it at once.
Checked by build and screenshot only; the drag itself was not clicked by me.

## Round 11 — side tabs and Auto Hide (reference: attachments/picture-45..47)

The side stripe now has only "Session Manager" and "Active Sessions" (the Command Manager tab, its View check box,
its Go-to item and Alt+J are gone). Auto Hide is on by default: the Session Manager / Active Sessions panel floats over the
session window and hides at the first click anywhere outside it (menus and pop-ups on top do not count; clicking the side
tab still toggles it). The pin button docks the panel (the window then shrinks beside it) and a second click returns to
Auto Hide. Checked by build and screenshot; the click-outside itself was not clicked by me.

## Round 12 — Delete key and confirmations (reference: attachments/picture-48.png)

Session Manager: the Delete key (pointer over the manager, no text box focused) now deletes the selected session or folder;
before, a focused tree row blocked it. Deleting by key, toolbar, context menu or Edit asks "Are you sure you want to delete
the selected item(s)?" with Yes / No (Enter = Yes, ESC = No) — the text is copied from the original's message box.
Estimated wording (not seen in the original): asking before Disconnect, Disconnect All, closing a connected tab, Close
Others / To the Right / All, and Exit (menu or the window's own close button) while sessions are connected. Closing a
tab that is not connected still closes at once.

## Round 13 — Delete key fix, no default sessions (reference: attachments/picture-49.png)

The Delete key failed because a clicked tree row keeps keyboard focus, and the check treated any focused widget as a
text box. It now only holds back when a text box is focused. The Default_LocalShell and Default_Serial rows are no longer
listed in the Session Manager (the original starts with none); the tree shows only the person's own folders and sessions.

## Round 14 — keys typed with AltGr, missing glyphs (person's test report)

"/" never reached the shell: on keyboards where "/" (also "?", "@", "\", "{" ...) needs AltGr, Windows reports AltGr as
Ctrl+Alt, the window library then drops the character and our key code treated it as a Ctrl+Alt shortcut. Now, for a
Ctrl+Alt key press, the terminal asks Windows (ToUnicode) which character that key gives with AltGr and sends it; layouts
without AltGr still get the control code. Glyphs: Segoe UI Symbol, Microsoft YaHei, MS Gothic, Malgun Gothic and Microsoft
JhengHei are added behind the built-in fonts (when present), so check marks, arrows and Chinese / Japanese / Korean text
draw instead of boxes. Shift+PgUp still scrolls our scrollback (not confirmed against the original). Function keys and Tab
were not re-tested. Checked by screenshot (glyphs) only; the AltGr key itself could not be pressed by me.

## Round 15 — keyboard focus (person's test report)

If a menu button kept the keyboard focus (menu closed with ESC, the window just started, Exit highlighted) the typed keys
went nowhere. Now, when no menu, dialog or text box is in use and a session is open, the next key press takes the focus
back from that button and hands it to the active session window. Not checked by pressing keys (screenshot only).

## Round 16 — selection, less, Home/End (person's test report)

Selecting by dragging: the anchor is now the cell where the button went down (egui only reports the drag after the pointer
has moved a few pixels, which cut the first characters of a quick drag), and the end follows the pointer up to the release.
Prompt shown twice after leaving `less`: the screen parser restores the shell screen correctly (unit test with the
full-screen enter / leave sequence passes), so this was not reproduced and nothing was changed for it. Home / End: they now
send ESC[1~ and ESC[4~ like the original (ESC O H / ESC O F in application cursor mode); the check box "Home and End send
ESC[1~ and ESC[4~" on Terminal > Emulation > Mapped Keys turns it back to ESC[H / ESC[F. Not tried against real less / vim
(no server here); `less` on a TERM=xterm host may ignore ESC[1~, which is why the choice is left to the person.

## Round 17 — Auto Hide after connecting

While the Session Manager is in Auto Hide (not pinned), opening any session (double-click, Connect, quick connect, local
shell, connect bar) hides the panel at once, as in the original. A pinned (docked) panel stays. Checked by screenshot
(panel open, session started: panel gone); the double-click itself was not clicked by me.

## Round 18 — menus compared with the person's test table

Edit: Copy shows Ctrl+Ins, Paste shows Shift+Ins; "Go to Session Window" is now "Go to Active Sessions (Alt+Z)"; "Go to
Command Manager (Alt+J)" is back. View: "Session Tabs" and "Command Manager" check boxes added, "Full Screen" shows Alt+Enter.
The Command Manager side tab is off by default (as in the original's picture-46) and appears only when View > Command Manager
(or Edit > Go to Command Manager / Alt+J) turns it on; the setting was renamed so an older saved "on" does not bring it back.
Home / End already follow the original (ESC[1~ / ESC[4~). Shift+PgUp is left scrolling the local history (the table says the
difference is the original's). Not done: the original's first-time "Right-click action" dialog (its text was never captured),
the Horizontal Scroll Bar (stays greyed), and the Ctrl+U-after-paste question (inconclusive). Hex View and Always on Top stay
in View as extras.

## Round 19 — session window colour (reference: attachments/picture-85/86.png)

The original's session window is pure black after connecting; ours was dark grey (#1e1e1e). "System Color Scheme" (the default
for every session) is now text #f0f0f0 on black #000000. The grey of the menus, toolbar and side panels is unchanged.
Checked by screenshot (local shell): black background.

## Round 20 — fonts, sizes and colour intensities (measured on attachments/picture-87.png, 1920x1032)

Pixels measured in the original's session window: background 0,0,0; default text 255,255,255; bright (bold) colours are
the VGA ones — red 255,85,85, green 85,255,85, blue 85,85,255 (we had 255,0,0 / 0,255,0 / 0,0,255). So the bright half of the
palette is now 555555 ff5555 55ff55 ffff55 5555ff ff55ff 55ffff ffffff (the normal half is unchanged: not measurable there) and
the "System Color Scheme" text is white. Letter shapes and spacing point to Consolas with rows 13 px high and columns about
7.1 px wide (153 columns in the original's window): the session window now uses Consolas, font size 13 (was Hack 14) and rows as
tall as the font size; menus, dialogs and trees use Segoe UI at 12 px (the original's 9 pt). Saved sessions that still had the
old size 14 are moved to 13 when the settings load. Estimated: the Consolas / Segoe UI identification (judged from shapes, the
original was not run), and the normal-colour half of the palette.

## Round 21 — Confirm Multi-line Paste (reference: attachments/picture-88.png)

Pasting text with more than one line (Edit > Paste, Shift+Ins, Ctrl+V with CUA) opens "Confirm Multi-line Paste": the
clipboard text in a read-only box, Paste (Enter) and Cancel (ESC), "Do not show this dialog again" (stops the question
from then on) and "Override line send delay: [5] milliseconds" (sends one line per delay; the value and check are kept in the
settings). Nothing is sent until Paste is pressed. A single line, or a line with just a trailing line break, pastes at once.
A program that asked for bracketed paste still gets the text in one piece. Estimated: the default 5 ms is read from the
picture; the answer-keeping rules (what "Do not show" stores) were not seen. Checked by screenshot of the window; the real
clipboard paste was not clicked by me.

## Round 22 — Logon Actions and "Expect/Send Properties" (reference: attachments/picture-89..91.png)

The Add... / Edit... window of Logon Actions is now the original's "Expect/Send Properties": groups Expect (Expect:), Send
(Send:, Hide, Send trailing carriage return), Credentials (optional) with a list starting at "<not set>", and the Commands list
(\r \n \b \v \p \s \w \e \t \\), OK / Cancel on the right; ESC closes it. A chosen credential makes \s the user name and \w the
saved password when the session opens (the saved session itself is not changed). \b (backspace) now works in every send text;
\v (paste clipboard) and \p (pause one second) are listed as in the original but not applied yet. The Logon Actions page has the
bigger Expect / Send / Hide / Send trail... list with its column titles on top, Add / Edit / Delete buttons, and "Logon script:"
with a browse button and an Arguments box (the arguments are stored; the script runner does not use them yet). Checked by
screenshots of both windows; not clicked through.

## Round 23 — Command Window and Command Manager (reference: attachments/picture-92..95.png)

Command Window (View > Command Window, Alt+G): a blue line on top, the title "Send commands to active session" (all /
visible sessions by choice) with a dotted grip and a close button, and a tall multi-line box with a scroll bar. Enter sends the
text with a carriage return and clears the box. Right-click menu as in the original: Undo (greyed here), Cut, Copy, Paste, Delete,
Select All, Send Characters Immediately (Ctrl+Shift+I: every typed character goes out at once), Show Command Window Title Bar
(Ctrl+Shift+W), Send Commands to > Active Session (Ctrl+Shift+T) / All Sessions (Ctrl+Shift+S) / Selected Tab Group(s) (greyed,
no tab groups) / Visible Sessions (Ctrl+Shift+B), Properties (opens Global Options; the original's own properties window was not seen).
Command Manager (side tab, View > Command Manager, Alt+J / Alt+Y): its own list of commands, no longer the button bar. Tool
row: send, add, cut, copy, paste, delete, properties, new folder; a filter box "Filter by folder/command name <Alt+Y>"; a tree
"Commands" with folders first; double-click sends. "Command Properties": Function "Send String", Disable tooltip, the string,
Label, Icon colour (Green, Red, Blue, Yellow, Orange, Purple, Gray, Black — only Green was seen), Description and the Send String
commands list; OK stays off until label and string are filled; Clear empties the fields. Deleting asks first. Not built: other
functions than Send String, drag and drop in the command tree, the real handling of \v (paste) and \p (pause) in sent strings.
Also: the floating Session Manager now sits behind dialogs instead of in front of them.

## Round 24 — Command Manager menu, keys and escapes (reference: attachments/picture-96..98.png)

Right-click menu as in the original: Send Command (Enter), Send Command to > Active / All / Visible Sessions, Cut (Ctrl+X),
Copy (Ctrl+C), Paste (Ctrl+V, greyed without something cut or copied), Delete (Del), Rename (F2), Expand All Folders,
Collapse All Folders, Properties (Alt+Enter). The same keys work with the pointer over the Command Manager. Command strings
are sent with their escapes: \r carriage return, \n line feed, \t tab, \e escape, \b backspace, \\ backslash, and now \p, which
waits one second before the rest goes out (the pieces are queued per session). Still not applied: \v (paste the clipboard),
and escapes typed in the bottom Command Window (it sends the text as typed, as far as the original's window is known).
Folders can be renamed; their commands follow. Checked by build and screenshot only (menus and keys not clicked by me).

## Round 25 — Command Window line and Script > Run (reference: attachments/picture-99..102.png)

The line on top of the Command Window is blue only while its text box has the keyboard focus (as in picture-92) and thin grey
otherwise (pictures 99 and 100). Script > Run... now opens the Windows file dialog at once, titled "Select Script to Run", with
the type list "Script Files (*.vbs;*.vbe;*.js;*.pl;*.py;*.ps1;*.txt)" and "All Files (*.*)" (the original's own list was cut off in the
picture, so the exact extensions are estimated); the chosen file runs on the active session. The old typed-path box is gone.
Only the simple text script language (send / sendln / waitfor / sleep / connect) runs; VBScript, JScript and Python files
choose fine but are not executed as such. Checked by screenshot (grey line) and build; the dialog was not opened by me.

## Round 26 — drag files from Windows onto an SFTP session

Dropping files or folders from Windows onto a connected SFTP session uploads them to the remote folder the session is in: each
file is sent with `put "<path>"` and each folder with `put -r "<path>"` on the running sftp program. While files are held over
the window a blue hint shows. Dropping on a session that is not SFTP (or not connected) does nothing except a status message.
Not seen in the original: its transfer progress window and the question about overwriting (sftp's own messages show in the
session instead). Not tried with a real drop or server (build only).

## Round 27 — shortcuts and what is still missing for a full migration

New: Ctrl+Tab goes to the next session tab and Ctrl+Shift+Tab to the previous one (wrapping round); the key is taken before the
terminal sees it, so it no longer reaches the host. Shortcuts that already existed: Alt+1..9 (tab), Alt+Enter (full screen /
Properties in the manager), Alt+B / Alt+C / Alt+Q (connect, quick connect), Alt+Shift+L (local shell), Alt+M / Alt+Z / Alt+J (side
tabs), Alt+G / Alt+R / Alt+U / Alt+Y (command window, connect bar, keyword box, command filter), Alt+A (reconnect all), Alt+P (SFTP),
Ctrl+Shift+A (select all), Ctrl+= / Ctrl+- / Ctrl+0 (zoom), Ctrl+Ins / Shift+Ins (copy / paste), the Command Window's
Ctrl+Shift+I / W / T / S / B, Ctrl+D / Ctrl+F / F2 / Del / Ctrl+X / C / V in the managers. Other shortcuts of the original's
default keymap were not in any picture or in the decompiled menu file, so none were guessed.
Review of what is still not built (all stated before, gathered here): Kermit and Zmodem transfers; RDP, SSH1, Telnet/TLS and TAPI
protocols; VBScript / JScript / Python scripting (only a simple text script runs); the Change Configuration Passphrase; the
horizontal scroll bar; tab groups ("Selected Tab Group(s)", "Connect in New Tab Group" opens a normal tab); the first-time
"Right-click action" dialog; \v (paste clipboard) in command / expect strings; the transfer progress window for SFTP drops; the
Undo entry of the Command Window menu; and the exact look of every window of the original, which was only ever seen in pictures.
Checked by build; the Ctrl+Tab key was not pressed by me.

## Round 28 — fixes from the regression report (Yetty vs SecureCRT)

Fixed: BUG-01 tabs can be dragged to a new place (a blue marker shows where it lands; the active tab stays active). BUG-02
Clone Session / Connect SFTP Tab pass on the password typed at the "password:" prompt (kept in memory, handed on encrypted), so the
new tab does not ask again; if Session Options of that clone are later saved the encrypted password is saved with them.
BUG-04 Duplicate... asks for the new name (suggesting "name (copy)", "(copy 2)"... unique in the folder) and refuses a used name; pasted
copies get unique names too. BUG-05 the Port Forward window (and the other sub-windows: Map a Key, prompts, Expect/Send...)
came out behind the Session Options window when opened a second time, because the windowing library remembers the old stacking
order; they are now raised on every frame. BUG-06 the session log is flushed to disk at least every 300 ms (Windows showed a file
of 0 bytes while it was written); the file prompts (Log Session, Run...) now have Cancel and Browse... buttons.
DIV-07 tiled and cascaded windows have a title bar with the name, a blue one for the active window, and a close button.
DIV-09 the tab menu has Reset Name and Close Disconnected Tabs. DIV-10 Edit > Select All / Clear Scrollback / Clear Screen are
grey without a session.
Not changed (needs work not possible from the pictures or too large): host key and password dialogs (the ssh program of the system
asks in the terminal; a real dialog needs our own SSH library), "Public Key Authentication Failed" dialog, New Session Wizard, SSH1 /
Telnet-TLS / RDP / TAPI, SFTP tab placement next to its source tab, the keyboard-picture Keymap Editor, the larger Global Options,
the toolbar copy / paste / print buttons, menu icons, the Font Awesome glyphs, selection colour, the Button Bar creation UI and
the darker ANSI red (red is the original's palette value; kept).

## Round 29 — retest of the regression report

Confirmed fixed by the retest: tab drag, clone / SFTP without a new password, Duplicate window with unique names, log flushing,
tile titles. BUG-05 (Port Forward window behind Session Options) was NOT fixed by round 28: egui keeps the relative order of
windows raised in the same frame, and the parent dialog was raised too. Now only child dialogs are raised (the Session Options and
Quick Connect dialogs are not), so a child always comes out in front, also when opened a second time. The same cause probably hid the
Duplicate window in the first attempts. Also: an SFTP tab opens right after the tab it came from (Alt+P, Connect SFTP Tab, menu);
Script > Edit... opens "Select Script to Edit" and then the script in Notepad; "Receive ASCII..." shows a check mark while
receiving instead of a loose check box.
Still open: host key and password dialogs (BUG-03), Global Options and Keymap Editor contents (DIV-08), missing tab-menu items for
tab groups / new windows / SecureFX / Font, Tools > Export Public Key from Certificate, Open Scratchpad / Open File in the File menu,
protocols SSH1 / Telnet-TLS / RDP / TAPI, Quick Connect "Properties..." button instead of the gear icon, tofu for Font Awesome glyphs,
min / max buttons on tiled windows, `ls -l` permissions shown as `-rw-******` by sftp.exe of the system (not ours).

## Round 30 — SFTP permissions without asterisks (attachments/picture-168/169.png)

Cause: the sftp.exe of Windows OpenSSH formats the `ls -l` lines itself and cannot print the group and other bits, so they show as
`******` (and the link count as `?`). SecureCRT has its own SFTP client and shows them in full. Yetty's SFTP tabs now start the
sftp that comes with Git for Windows (Program Files\Git\usr\bin\sftp.exe, also the per-user install) when it is installed — it
prints the full permissions — and fall back to the Windows OpenSSH one otherwise (the asterisks then remain). Checked only that the
Git sftp starts from Yetty (a refused connection printed its message); no server was available to see a real listing. Not
identical to the original: its own listing layout (dates with years, column widths) comes from the program that runs.

## Round 31 — SFTP listing layout (attachments/picture-170/171.png)

The lines of `ls -l` in an SFTP tab are rewritten to the original's layout: no link-count column, dates as "Sep 22, 2026 20:45"
(day with two digits, year always), owner and group in columns, size right-aligned, and the listing sorted by name ignoring case
(1.sh, acr-delete.log, Apps, balabit.txt...). Only lines that look like a long listing line are changed; everything else, including
the prompt, passes through. A line that waits for the rest or a listing that nothing follows is shown after 250 ms. Limits: sftp
prints no time for files older than about six months, so those lines show "May 19, 2026" without the time (the original knows it);
a recent file's year is taken from the clock. The SFTP tab is named "SFTP-<host>" like the original. Unit-tested on sample lines; not
tried on a live server. Not copied: the original's "Default" tab-group bar under the terminal.

## Round 32 — host key / password windows, Port Forward in front, Global Options, Keymap Editor (retest items BUG-03, BUG-05, DIV-08)

BUG-03: when the system's ssh asks "...continue connecting (yes/no/[fingerprint])?" a window "New Host Key" opens (host name,
fingerprint, View Host Key, Accept & Save = answers yes, Cancel = answers no); when it asks "user@host's password:" (and no password is
saved) a window "Enter Secure Shell Password" opens (password box, Save password, OK, Skip, Cancel). Save password keeps it encrypted
in the session and in the matching saved session. The text of the question still appears in the terminal behind the window (the
original shows none). Not built: "Public Key Authentication Failed" (ssh does not say so without -v). Checked by screenshots of both
windows; no live ssh server here.
BUG-05: round 29 raised only child dialogs, but a dialog opened from Global Options was also affected; Global Options is now a parent
as well. Not re-checked on screen (needs a second opening by hand).
DIV-08: Global Options is a category dialog (General, Default Session, File Transfer, Terminal > Appearance / ANSI Color, Tabs, SSH2,
Firewall) with OK / Cancel; settings apply at once. ANSI Color edits the 16 colours (Normal and Bold columns, Reset) and they are
used by every session. Keymap Editor shows an on-screen keyboard: choose Ctrl / Alt / Shift, click a key (blue = already mapped),
choose Function (Send String, Send Hex String, Copy, Paste, Clear Screen, Send Break, Disconnect) and Argument, then Map or Unmap;
Save... / Load... keep a keymap as a file. The mapped keys are those of the default session (new sessions). Not the original's real
categories or look: its Global Options pages were never seen, only its names from the test report; Firewall has no entries but None,
"Tabs" only has show tabs.

## Round 33 — small window titles for all sub-windows (attachments/picture-179/180.png)

Port Forward, Map a Key, the question window of the file prompts (Log Session, Run...), Find, Button, Trace Options, Saved Credentials,
Manage Snapshots, Public-Key Assistant, Scratchpad, TFTP Server, Help, About and the Xmodem / Ymodem progress window were drawn
by the library's own window frame (big centred title, big close button). They now use the same frame as the main dialogs: a small
left-aligned title (12 px) with a small close button, ESC closes them (not the progress window). The library's Heading text style is also
12 px now. Window widths were chosen by eye. Checked by a screenshot of Port Forward; the other windows were not opened one by one.

## Round 34 — pending items of the third retest

BUG-03: the text of the host key question and of the "user@host's password:" line is now kept off the terminal (the logon logic
still reads it), so only the windows show. The public key failed window is still not built (ssh says nothing about it without -v).
DIV-07: tiled windows have minimise (folds to the title bar), maximise / restore (fills the area) and close buttons, and the tab bar
stays visible in tile and cascade modes. DIV-08: Global Options gained Keyboard (copy / paste hotkeys, Meta, backspace, Home / End,
button to the Keymap Editor), Mouse, Printing (header, footer) and Logging (default log file settings) next to the earlier categories;
its other categories in the original are not known. DIV-09: the tab menu has Clone in New Window, Send to New Window (starts another
Yetty window for the session and closes the tab), Font... (opens the Appearance page), Close with "Ctrl+F4" (and the key works);
"Send to New Tab Group" and "Close Tab Group" are shown greyed (no tab groups). DIV-10: File has Open Scratchpad and Open File...
(Open File shows the file in the scratchpad window); menu icons are not drawn. DIV-11: RDP is in the Protocol list; it starts the Windows
Remote Desktop client (mstsc) for host and port 3389 in its own window, no tab. SSH1, Telnet/TLS and TAPI are still missing (the
system's ssh has no SSH1; no TLS library). DIV-12: the "Show quick connect on startup" setting was renamed so it starts checked again
for everyone (an older saved "off" no longer applies). DIV-13: ANSI red (31) is now 212,42,42 instead of 160,0,0 — an estimate, the
original's value for it was not measured.

## Round 35 — pending items measured against the running SecureCRT (app/shots/crt_global_options, app/shots/crt_ref)

REG-01: not a regression. The original also starts on "Send commands to active session" (Global.ini "Initial Send Commands To
Value"=0); All Sessions is in the Command window's right-click menu (Send Commands to, Ctrl+Shift+S) in both programs. Added the
original's Global Options > Terminal > Advanced "Initially send commands to" (Active Session / All Sessions / Selected Tab Group(s) /
Visible Sessions) and the Selected Tab Group(s) target. DIV-07: the original's tiled sessions are windows with "name - Connected"
title bars and no tab bar (evidencia_11b), so the tab bar is hidden again in Tile / Cascade and the pane titles show the state.
DIV-08: Global Options now has the original's 18 pages and order (General: Configuration Paths, Auto Session, Credentials;
Terminal: Appearance > ANSI Color / Status Bar / Advanced, Tabs/Tiling, Printing, Web Browser, TFTP Server, Advanced; Firewall;
SSH2 > Agent Keys; SSH Host Keys). Keyboard, Mouse, Logging, Default Session and File Transfer are not Global Options pages in the
original (default sessions stay in Options > Configure). Options Yetty cannot honour are shown greyed. The original's Keymap Editor
has Map / Unmap / Save / Load (no Edit / Delete). DIV-09: the tab menu follows the original's order and items; tab groups are built
(Send to New Tab Group puts the tab in a new group to the right, each group with its own tab bar; Close Tab Group; Send Commands to
This Group; Window > Single Tab Group merges them; new tabs open in the active group). Save Session... stores the tab in the Session
Manager. Open SecureFX is greyed (separate product, greyed in the original here too). DIV-10: menu items have the original's icon
column (icons on Connect, Quick Connect, Connect in Tab/Tile, Connect Local Shell, Open Scratchpad, Reconnect, Disconnect, Copy,
Paste, Find, Session Options, Keymap Editor, Public-Key Assistant). DIV-11: Telnet/TLS (port 992, SChannel through native-tls,
certificate and host name checked against the Windows store) and the New Session Wizard (protocol, host / serial port, name and
description; Global Options "New session uses wizard"). SSH1 and TAPI left out by the person's decision (no SSH1 in the system's
OpenSSH, TAPI needs a modem). DIV-12: Quick Connect starts with "Show quick connect on startup" checked (verified). DIV-13: the ANSI
palette is the one the original draws (measured in its window: 000000 ff5555 55ff55 ffff55 5555ff ff55ff 55ffff bbbbbb, bright
555555 … ffffff); the ANSI Color page lists the original's 15 palettes.

## Round 35 — the Session Manager remembers which folders are open (attachments/picture-181.png)

The open / closed state of the Session Manager folders (and of the Sessions root) is kept in the settings file and put back when
the program starts, the panel is closed and opened again, or a session is connected; "Expand All Folders" / "Collapse All Folders"
are kept too. A folder that was never opened starts closed. The same is done for the Command Manager (its folders start open; the
closed ones are remembered). Folder state follows the folder path, so a renamed or moved folder starts closed. The state is written
with the other settings (when they are saved). Checked by a screenshot with a folder stored as open; not by closing and reopening.

## Round 36 — regression of the retest reports (RELATORIO_PENDENCIAS), live against SecureCRT and a WSL ssh server

Re-checked on the running program: REG-01 (Send Commands to > All Sessions reaches every tab), DIV-07 (tile titles
"name - Connected", no tab bar), DIV-08 (18 Global Options pages), DIV-09 (tab menu order and greying match the original with one
tab), DIV-10 (File: Open Scratchpad / Open File, icons), DIV-12, the password and host key windows, and the folder state kept
across a real restart. Defects found and fixed:
- Alt shortcuts typed their letter: the window library reports Alt+G / Alt+Q / Alt+M... also as text, so "g" reached the host
  (`echo Xgg`) or the focused box ("q" in the Command Window). The text event that follows an Alt key press (without Ctrl, so
  AltGr keeps working) is now dropped before anything reads it.
- Tile panes: minimise, maximise and close did nothing (the title bar's click area was registered after them and took their
  clicks). The bar is registered first; a pressed button also makes its pane active; the name is cut before the buttons.
- Authentication list (Quick Connect, Session Options): the check boxes could not be cleared (the row's click area covered them).
- Quick Connect "Save session" stored a second session with the same name each time; it now saves under the hostname, as the
  original (its file is `localhost.ini`), with " (1)", " (2)"... when the name is taken. The New Session Wizard refuses a name
  that already exists in the folder ("A session named ... already exists in this folder.").
- After a resize the pseudo console redraws its screen, so the hidden "user@host's password:" line (and the host key text) came
  back on the terminal (the residual seen in the retest's 59/60 pictures). Hidden lines are remembered and removed from redraws;
  ssh's "Warning: Permanently added ... known hosts." is hidden too (the original shows none).
- Quick Connect has the original's gear button instead of "Properties..." (Session Options keeps "Properties...").
- Tab menu: Reset Name is grey until the tab is renamed (host title changes no longer count, and no longer replace a name given
  by Rename); Reconnect / Disconnect / Session Options have no pictures there, as in the original.
- Sub-menu entries (Configure, Zoom, Send Commands to...) line up with the other items.
- Session Manager: sessions and folders are listed alphabetically (Global.ini "Sort Alphabetical"=1); Global Options > General
  "Session list sort order" (Alphabetically / Manually) and "Save expanded folder state for session list" now work.
Still different (not fixed): the "Public Key Authentication Failed" window — ssh only reports a refused key in its debug log, and
`ssh -E file` keeps that file locked while it runs (tested), so it cannot be read in time; the global ANSI Color page shows the
colours Yetty draws, while the original's "Standard" page shows its VGA palette and draws the colour scheme's (Global.ini
"Color Scheme Overrides Ansi Color"=1); "Export Public Key from Certificate...", Kermit, Zmodem upload list, Change Configuration
Passphrase, the Command Window's Undo and menu mnemonics (underlined letters) are not built. Session list label "Manually" is an
estimate (the original's second choice was not seen).

## Round 8 — keyboard, pointer, PublicKey and the pages of reference/picture-3..5

ESC closes the dialog or sub-window that is on top (and is not sent to the terminal); labels no longer show the text
cursor. PublicKey Properties: global or session identity file (browse, keys found in .ssh, type and fingerprint from
ssh-keygen -l), ssh-agent on/off, "use only this identity file" (IdentitiesOnly), stored passphrase (encrypted, answered
at "Enter passphrase for key"), Copy Public Key, Create Identity File. Terminal page: audio/visual bell, close / clear on
disconnect, auto reconnect, scroll to bottom on output / keypress, wheel to remote, anti-idle string and NO-OP, word
delimiters. Emulation page: terminal, ANSI colour, xterm-256color, use colour scheme, logical rows/columns, On resize
(synchronize / scale the font / retain), scrollback. Mapped Keys: Key Combination / Function / Argument table with
"Map a Key...", Edit, Delete; Backspace sends delete, Delete sends backspace, CUA hotkeys. Window page: vertical scroll bar,
disable resize, always on top, title bar. Not built: Jump scroll, Minimize drawing, Shift forces local mouse, Position cursor
on click, alternate keyboard emulation, horizontal scroll bar, button bar choice, initial position.

## Round 37 — the pending items of Round 36 (PENDENCIAS.md)

- Public Key Authentication Failed: with an identity file chosen, ssh runs with -v; the "debug1:" lines (and "Authenticated to",
  "Transferred:") are kept off the screen unless Trace Options asks for them. "Offering public key" of that file followed by
  "Authentications that can continue" without "Server accepts key" opens the window. Its wording is an estimate.
- Global Options > ANSI Color: "Color scheme overrides ANSI color" (on by default, Global.ini "Color Scheme Overrides Ansi Color"=1):
  with a colour scheme in use ANSI black / white are drawn as the scheme's background / text. How far the original goes is not known.
- Tools > Export Public Key from Certificate: X.509 certificate (PEM, DER, PFX / P12 through Windows without password) to an OpenSSH
  line for RSA, ECDSA and Ed25519. Menu order and mnemonic from Menus/default.mnu.
- Kermit and Zmodem: new engines (`kermit.rs`, `zmodem.rs`) behind the same progress window as Xmodem / Ymodem. Zmodem upload list and
  Start Zmodem Upload send the session's upload command ("rz") first; a host's sz request starts a download by itself (Session
  Options > X/Y/Zmodem can turn that off). Loopback tests only.
- Change Configuration Passphrase: saved passwords and key passphrases are encrypted with DPAPI plus the passphrase as entropy
  ("dpapi2:"); a verifier in the settings is checked when Yetty starts, before any session opens (start-up sessions wait for it).
  Empty new passphrase removes it. Forgetting it loses the saved passwords.
- Menu mnemonics: captions and letters read from the original's Menus/default.mnu. Alt+letter opens the menu bar entries (not when
  the session uses Alt as Meta), letters choose items of an open menu, underlines show while Alt is down or a menu is open.
- Global Options > General: "Sort alphabetically" check box (the original's wording, read from SecureCRT.exe) instead of a list.
- Terminal options: jump scroll (output drawn every 120 ms), minimize drawing (every 50 ms), mouse reporting to programs that ask
  for it (X10, normal, button and any motion; default and SGR encoding; wheel as buttons 4 / 5) with "Shift forces local mouse",
  position cursor on click (arrow keys), alternate keyboard (None / Linux / Xterm R5-VT220 for F1-F5), horizontal scroll bar
  (fixed-size terminal wider than the window), named button bars (context menu of the plus button; chosen in Session Options >
  Window) and initial position of the window. Mouse reporting is new: before, no click reached the program.

## Round 38 — main screens compared with the running SecureCRT 9.7 (side by side, same window size)

Measured / read on screen: panels, menu bar, toolbar and tab bar are `1f1f1f`; the status bar `333333` with no separators between its fields;
inactive tab `2e2e2e`, selected tab `3d3d3d` with a 3 px top line (blue when it has the focus, grey otherwise) and a bold green check mark,
close button only on the selected tab; a blue line under the tab bar; the session window has a light frame (`a0a0a0`, then `696969`); the
vertical scroll bar is 17 px wide with an arrow button at each end (track `2d2d2d`, thumb `4e4e4e`); a 5 px `3d3d3d` splitter above the Command
Window, whose box has a 1 px `9a9a9a` border; the "Default" tab-group button under the session window (menu: the group list and Send to New Tab
Group); no separator lines between menu bar / toolbar / tab bar; pop-up menus `2e2e2e` with a `676767` border and round corners, text `d6d6d6`,
underlined mnemonic letters only after Alt was pressed (not for a menu opened with the pointer).
Icons: the toolbar, the Session Manager tool row, the side-panel buttons and the menu icons use Segoe MDL2 Assets (`segmdl2.ttf`, loaded as a
fallback font) in white, with the original's order (Connect, Quick Connect, Local Shell, Reconnect, Disconnect | Copy, Paste, Find | Print |
Session Options, Keymap, Public Key | Help); the first button is Connect (it was the Session Manager toggle). No session prints
"Connecting to ... " into the terminal any more (the original shows it only in the status bar).
Dialogs follow the theme: on the dark theme their windows use the same colour as the main window (`1f1f1f`; pop-up menus keep `2e2e2e`) and
they are not a difference from the original. Field captions are left-aligned; frame strokes come from the theme.
Global Options > General: "Session list sort order" is a list with Alphabetically / Logically / Manually (read in the original's window; the
Round 37 "Sort alphabetically" check box was wrong). Logically compares numbers by value.
Still different: blue check boxes, group titles drawn inside the border, tab scroll arrows, status bar size grip, the tree's +/- boxes and dotted
lines in the Session Manager, window icon.

## Round 39 — Linux port

Goal: the Windows program, unchanged in look and behaviour, builds and runs on Linux. System-specific code is behind `cfg(windows)` /
`cfg(not(windows))`; the Windows paths were left as they were (checked with `cargo check --target x86_64-pc-windows-msvc`, not run).
State of every item: PENDENCIAS.md, "Versão Linux".
- Build: `native-tls` needs OpenSSL on Linux; `openssl` with `vendored` (Linux only) links it statically. Windows still uses Schannel.
- Paths: `config::home_dir()` (`HOME` / `USERPROFILE`), `config::data_dir()` (`$XDG_CONFIG_HOME/yetty` / `%APPDATA%\Yetty`).
- Saved passwords: DPAPI has no Linux equivalent, and the old stub stored them in clear text. `config::linux_secret`: ChaCha20-Poly1305,
  random 32-byte key in `<data dir>/master.key` (mode 0600, directory 0700, created once); with a configuration passphrase the key is
  PBKDF2-HMAC-SHA256(passphrase, master key, 100 000). Stored as `lkey:` / `lkey2:` + hex(nonce ‖ ciphertext). Values of the other system
  (`dpapi:` on Linux, `lkey:` on Windows) decrypt to "" and are never used as plain text.
- Programs: `find_program` drops `.exe` off Windows; local shell = `$SHELL`, command field split like a shell (quotes, `\`) off Windows;
  `conn::rdp_command` (mstsc / xfreerdp3 / xfreerdp / remmina / rdesktop); `config::print_file` (notepad /p / lp), `edit_file`
  (notepad / xdg-open), `create_desktop_shortcut` (.lnk through PowerShell / .desktop), `open_console` (cmd / first terminal emulator),
  `open_keygen_console`; PFX / P12 read through the linked OpenSSL; serial ports `/dev/ttyUSB0..7` as the list when none is found.
- Dialogs: `nativedlg.rs` keeps the Win32 calls on Windows; elsewhere `rfd` (desktop portal).
- Fonts (`sysfonts.rs`): Windows loads the same files as before. Linux: a font the person installed (`consola.ttf`, `segoeui.ttf`,
  `segmdl2.ttf`, in a system font folder or `<data dir>/fonts`) wins; otherwise bundled free fonts with the same metrics:
  Selawik (Segoe UI), `yetty-icons.ttf` (Fluent System Icons placed at the Segoe MDL2 code points, `tools/make-icons-font.py`), `yetty-mono.ttf`
  (Hack with unitsPerEm 2048 → 2243, so cells are 0.550 em like Consolas, `tools/make-mono-font.py`). Symbols / CJK from Noto, DejaVu, WenQuanYi.
  Checked against `docs/images/tela-inicial.png` and `terminal-local.png`: menu bar words within 3 px, 7.2 px per terminal column (124 columns
  where plain Hack gave 113), toolbar icons same size and row.
- Status bar Caps / Num Lock from `/sys/class/leds`. View > Window Transparency works off Windows through `winalpha.rs` (window created with an alpha channel; every shape of the frame faded by 200/255). Window `app_id` "yetty" (Linux only).
- Still to do: real ssh / SFTP login, keyboard on the real window, portal dialogs, RDP / print / serial, transparency, AppImage, native `sz` /
  `rz` / `gkermit` tests.

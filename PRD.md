# PRD — Tmux GUI Desktop & Web



## 1. Ringkasan Produk



### 1.1 Nama sementara



**Tmux GUI**



Nama repository sementara dalam PRD:



```text

tmux-gui

```



Nama ini dapat diganti kemudian tanpa mengubah arsitektur.



### 1.2 Tujuan



Membuat GUI untuk `tmux` agar pengguna tidak perlu mengingat command dan shortcut tmux.



Aplikasi **bukan pengganti tmux** dan bukan backend terminal baru.



`tmux` tetap menjadi:



- pemilik session;

- pemilik window;

- pemilik pane;

- pemilik proses terminal;

- pemilik layout;

- source of truth seluruh state terminal.



Aplikasi hanya memberikan lapisan GUI di atas tmux.



Terminal tetap dapat:



- dibuat melalui tmux CLI;

- diubah melalui tmux CLI;

- dibuka melalui tmux CLI;

- diubah melalui aplikasi GUI.



Perubahan dari kedua arah harus saling terlihat.



---



# 2. Prinsip Arsitektur



## 2.1 Source code



Repository dibagi menjadi:



```text

tmux-gui/

├── be/

├── fe/

├── desktop/

├── scripts/

├── Makefile

└── README.md

```



### Backend



```text

be/

```



Menggunakan Go.



Tanggung jawab:



- menjalankan tmux control mode;

- membaca state tmux;

- menjalankan command tmux;

- streaming output terminal;

- menerima input terminal;

- menyediakan REST API;

- menyediakan WebSocket;

- serve frontend production;

- menjadi backend bagi Electron.



### Frontend



```text

fe/

```



Menggunakan:



- Vite;

- React;

- TypeScript;

- Tailwind;

- shadcn/ui;

- xterm.js.



Frontend **tidak memiliki backend terminal sendiri**.



xterm.js hanya digunakan sebagai:



- terminal renderer;

- keyboard input collector;

- selection;

- clipboard;

- scrollback display.



### Desktop



```text

desktop/

```



Menggunakan Electron.



Electron bertanggung jawab untuk:



- menjalankan Go backend;

- membuka window desktop;

- membuka URL Go backend;

- lifecycle backend;

- native window controls;

- packaging Linux.



Electron tidak berisi business logic tmux.



---



# 3. Model Deployment



## 3.1 Development Web



```text

Browser

   │

   ▼

Vite :14102

   │

   │ proxy /api

   ▼

Go :14101

   │

   ▼

tmux

```



FE tetap menggunakan URL relatif:



```text

/api/...

```



Vite melakukan proxy ke backend.



Tidak perlu CORS.



---



## 3.2 Production Web



FE dibuild langsung ke folder embed backend.



```text

Browser

   │

   ▼

Go Backend

 ├── /

 │    └── embedded React build

 │

 ├── /api/*

 │

 └── /api/ws

       │

       ▼

      tmux

```



Dengan demikian deploy cukup:



```bash

./tmux-gui-server

```



Tidak ada:



```text

nginx untuk FE

node server

vite preview

frontend service terpisah

```



Go menjadi satu-satunya server.



---



## 3.3 Production Electron



```text

Electron

   │

   ├── spawn tmux-gui-server --port 0

   │

   └── BrowserWindow

          │

          ▼

   http://127.0.0.1:<dynamic-port>

          │

          ▼

    Go Backend

       │

       ├── embedded FE

       └── tmux

```



Ini sengaja sedikit lebih sederhana daripada menjadikan Electron serve file FE sendiri.



Frontend desktop dan frontend web benar-benar sama.



---



# 4. Requirement Sistem



## 4.1 Operating System



V1 hanya:



```text

Linux

```



Tidak mendukung:



- Windows;

- WSL;

- macOS.



Backend saat startup harus mengecek:



```bash

runtime.GOOS == "linux"

```



Jika bukan Linux:



```text

Unsupported operating system.

Tmux GUI currently supports Linux only.

```



---



## 4.2 Dependency Runtime



Minimum:



```text

tmux

```



Backend melakukan:



```bash

tmux -V

```



ketika startup.



Jika tmux tidak ada:



```text

Tmux is not installed.

```



UI menampilkan halaman error yang jelas.



Target kompatibilitas awal:



```text

tmux >= 3.2

```



Testing terutama dilakukan terhadap:



```text

3.3+

3.4+

3.5+

3.6+

3.7+

```



---



# 5. Prinsip Integrasi Tmux



## 5.1 Jangan membuat database session



Tidak ada tabel:



```text

sessions

windows

panes

terminals

```



Tidak ada SQLite hanya untuk menduplikasi state tmux.



Semua state dibaca dari tmux.



Contoh:



```bash

tmux list-sessions

tmux list-windows

tmux list-panes

```



---



# 6. Stable Identifier



Backend jangan menggunakan:



```text

window index :1

pane index .0

```



sebagai identifier internal.



Gunakan:



```text

session = nama session

window  = @N

pane    = %N

```



Contoh:



```text

dev

@3

%12

```



Frontend menyimpan ID tersebut.



Contoh model:



```ts

interface TmuxPane {

  id: "%12"

  index: 1

}



interface TmuxWindow {

  id: "@3"

  index: 2

}

```



`index` hanya untuk display.



Semua command menggunakan stable ID.



---



# 7. Tmux Control Mode



## 7.1 Persistent connection



Terminal aktif menggunakan:



```bash

tmux -CC attach-session -t <session>

```



Backend membuka stdin/stdout process tersebut.



Struktur:



```text

Frontend

   │

WebSocket

   │

   ▼

Go TmuxMonitor

   │

stdin/stdout

   │

   ▼

tmux -CC

```



---



# 8. Aturan Menjalankan Command Tmux



## 8.1 Mutating command



Command yang mengubah tmux harus dikirim melalui control mode apabila control mode sedang tersedia.



Contoh:



```text

split-window

kill-pane

resize-pane

select-pane

rename-window

kill-window

select-layout

swap-pane

break-pane

send-keys

```



Alur:



```text

UI

 ↓

WebSocket

 ↓

Go command handler

 ↓

TmuxMonitor

 ↓

control mode stdin

 ↓

tmux

```



JANGAN:



```go

exec.Command("tmux", "split-window")

```



untuk command mutation normal ketika control-mode client sedang berjalan.



---



# 9. Read-only Query



Read-only query boleh menggunakan one-shot command.



Contoh:



```bash

tmux list-sessions

tmux list-windows

tmux list-panes

tmux display-message

tmux show-options

tmux capture-pane

```



Digunakan untuk:



- bootstrap initial state;

- recovery;

- sidebar tree;

- history terminal;

- resync.



---



# 10. Membuat Session Pertama



Edge case:



```text

tmux server belum ada

```



Maka control mode belum mungkin diattach.



Backend boleh bootstrap dengan:



```bash

tmux new-session -d -s <name>

```



Setelah session tersedia, control mode langsung dibuat.



---



# 11. New Window Compatibility



Implementasi `new-window` harus menggunakan compatibility helper.



Default:



```text

split-window

+

break-pane

```



Contoh konsep:



```bash

splitw -d -P

breakp

```



daripada bergantung langsung pada `new-window`.



Semua implementasi diletakkan dalam:



```text

tmux.CommandService.CreateWindow()

```



Frontend tidak perlu mengetahui workaround ini.



---



# 12. Data Model Backend



## Session



```go

type Session struct {

    Name      string

    Windows   int

    Attached  int

    CreatedAt int64

    Width     int

    Height    int

}

```



## Window



```go

type Window struct {

    ID        string

    Index     int

    Name      string

    Active    bool

    Panes     int

    Width     int

    Height    int

    Layout    string

}

```



## Pane



```go

type Pane struct {

    ID             string

    Index          int

    WindowID       string



    Active         bool

    Zoomed         bool



    Left           int

    Top            int

    Width          int

    Height         int



    PID            int

    CurrentCommand string

    CurrentPath    string

    Title          string

}

```



---



# 13. Layout Pane



Frontend tidak perlu mencoba menerjemahkan layout tmux menjadi bootstrap grid manual.



Backend membaca:



```text

pane_left

pane_top

pane_width

pane_height

window_width

window_height

```



Kemudian pane dirender secara proporsional.



Contoh:



```text

Window

┌────────────────────────────┐

│                            │

│         pane %1            │

│                            │

├──────────────┬─────────────┤

│   pane %2    │   pane %3   │

└──────────────┴─────────────┘

```



Frontend menggunakan posisi relatif berdasarkan geometry tmux.



Dengan cara ini layout kompleks tetap dapat dirender.



---



# 14. Fitur Utama — Session



Sidebar utama menampilkan:



```text

Sessions

├── dev

├── production

└── logs

```



Fitur session:



### Create Session



Form:



```text

Name *

Working directory

Initial command

```



Name wajib.



Validation terhadap nama session tmux.



Action:



```text

Create

```



### Rename Session



Context menu:



```text

Rename

```



### Kill Session



Context menu:



```text

Kill Session

```



Harus menggunakan confirmation dialog.



### Switch Session



Klik session.



Frontend:



1. disconnect control connection lama;

2. connect ulang ke target session;

3. request snapshot;

4. render window aktif.



### Session yang dibuat dari CLI



Contoh user menjalankan:



```bash

tmux new -s backend

```



Maka sidebar GUI harus memperlihatkan:



```text

backend

```



tanpa restart aplikasi.



---



# 15. Fitur Window



Di dalam session:



```text

dev

├── 0: editor

├── 1: server

└── 2: database

```



## Window actions



GUI menyediakan:



```text

New Window

Rename

Close

Next

Previous

Move Left

Move Right

```



Context menu:



```text

Rename Window

Move Left

Move Right

Break Active Pane

Kill Window

```



---



# 16. Window Layout Preset



Toolbar menyediakan:



```text

Even Horizontal

Even Vertical

Main Horizontal

Main Vertical

Tiled

Next Layout

```



Backend menjalankan:



```text

select-layout

```



---



# 17. Fitur Pane



Setiap pane mempunyai toolbar/context menu.



Actions:



```text

Split Right

Split Down

Kill Pane

Zoom

Rename Pane

Swap Pane

Break To Window

Join Pane

```



---



# 18. Split Pane



Tombol:



```text

Split Right

```



menghasilkan horizontal tmux split.



Tombol:



```text

Split Down

```



menghasilkan vertical tmux split.



Target harus menggunakan:



```text

%pane_id

```



bukan current active pane secara implicit.



---



# 19. Resize Pane Menggunakan Mouse



Divider pane dapat di-drag.



Contoh:



```text

pane A │ pane B

       ↑

      drag

```



Frontend menghitung perubahan pixel.



Kemudian dikonversi menjadi jumlah cell.



Contoh:



```text

delta pixels = 40

cell width   = 8

delta cols   = 5

```



Backend menerima:



```json

{

  "pane_id": "%12",

  "direction": "R",

  "amount": 5

}

```



Kemudian:



```text

resizep -t %12 -R 5

```



Resize harus diberi throttle sekitar:



```text

30-60 ms

```



agar drag tidak menghasilkan ratusan command.



---



# 20. Zoom Pane



Double click pane header atau tombol:



```text

Zoom

```



menjalankan:



```text

resize-pane -Z

```



Pane memenuhi workspace.



Action berikutnya mengembalikan layout.



---



# 21. Terminal Renderer



Gunakan:



```text

@xterm/xterm

```



Optional addons:



```text

@xterm/addon-fit

@xterm/addon-web-links

```



Xterm hanya bertanggung jawab untuk presentation.



Tidak ada:



```text

node-pty

shell spawning di FE

terminal session buatan aplikasi

```



---



# 22. Terminal Input



Input keyboard dari:



```ts

terminal.onData()

```



dikirim melalui WebSocket.



Contoh:



```json

{

  "type": "terminal.input",

  "paneId": "%12",

  "data": "..."

}

```



Input tidak dikirim satu command tmux per karakter.



Backend mempunyai:



```text

InputBatcher

```



Batch window sekitar:



```text

8-16 ms

```



atau maksimum misalnya:



```text

4 KB

```



Kemudian input diubah menjadi byte-safe tmux input.



Gunakan mekanisme yang tidak bergantung pada shell quoting.



Preferred:



```text

send-keys -H

```



dengan byte hex.



Contoh:



```text

hello

```



menjadi byte UTF-8 lalu dikirim sebagai hex.



Ini juga mengakomodasi:



- arrow keys;

- Ctrl sequences;

- ESC;

- function keys;

- UTF-8;

- mouse escape sequence.



---



# 23. Terminal Output



Control mode menghasilkan event seperti:



```text

%output

```



Backend parse payload tersebut.



Kemudian:



```json

{

  "type": "terminal.output",

  "paneId": "%12",

  "data": "..."

}

```



dikirim melalui WebSocket.



Frontend:



```ts

terminal.write(data)

```



---



# 24. Initial Terminal Snapshot



Ketika frontend baru membuka pane, `%output` sebelumnya sudah tidak tersedia.



Maka backend mengambil snapshot menggunakan:



```bash

tmux capture-pane

```



Contoh:



```text

capture-pane

-p

-e

-J

-t %12

```



Dengan history terbatas.



Default:



```text

2000 lines

```



configurable.



Setelah snapshot dikirim:



```text

snapshot

→

live %output

```



---



# 25. Realtime External Changes



Contoh user membuka terminal biasa dan menjalankan:



```bash

tmux split-window

```



GUI harus berubah.



Contoh:



```bash

tmux kill-pane -t %4

```



GUI harus menghapus pane.



Sumber detection:



1. tmux control-mode event;

2. debounced metadata resync;

3. periodic fallback refresh.



Fallback polling:



```text

1500 ms

```



hanya untuk tree metadata.



Output terminal tidak menggunakan polling.



---



# 26. WebSocket Protocol



Endpoint:



```text

GET /api/ws?session=<session>

```



## Client → Server



### hello



```json

{

  "type": "hello",

  "cols": 160,

  "rows": 48

}

```



### terminal.input



```json

{

  "type": "terminal.input",

  "paneId": "%3",

  "data": "..."

}

```



### terminal.resize



```json

{

  "type": "terminal.resize",

  "cols": 160,

  "rows": 48

}

```



### pane.select



```json

{

  "type": "pane.select",

  "paneId": "%3"

}

```



### pane.split



```json

{

  "type": "pane.split",

  "paneId": "%3",

  "direction": "horizontal"

}

```



### pane.resize



```json

{

  "type": "pane.resize",

  "paneId": "%3",

  "direction": "R",

  "amount": 4

}

```



### pane.kill



```json

{

  "type": "pane.kill",

  "paneId": "%3"

}

```



### pane.zoom



### pane.break



### pane.swap



### window.select



### window.create



### window.rename



### window.kill



### window.layout



### session.create



### session.rename



### session.kill



### state.resync



---



# 27. Server → Client Protocol



## connection.ready



```json

{

  "type": "connection.ready",

  "session": "dev"

}

```



## state.snapshot



Berisi:



```text

session

windows

panes

activeWindow

activePane

```



## state.delta



Perubahan metadata incremental.



## terminal.snapshot



```json

{

  "type": "terminal.snapshot",

  "paneId": "%2",

  "data": "..."

}

```



## terminal.output



## command.success



## command.error



## tmux.disconnected



## tmux.reconnecting



## server.error



---



# 28. Command Correlation



Setiap GUI action mempunyai:



```text

requestId

```



Contoh:



```json

{

  "type": "pane.kill",

  "requestId": "uuid",

  "paneId": "%12"

}

```



Backend mengirim:



```json

{

  "type": "command.success",

  "requestId": "uuid"

}

```



atau:



```json

{

  "type": "command.error",

  "requestId": "uuid",

  "message": "pane not found"

}

```



---



# 29. Backend Control Mode Parser



Parser minimal harus mengenali:



```text

%begin

%end

%error



%output



%layout-change

%window-add

%window-close

%window-renamed



%session-changed

%sessions-changed



%pane-mode-changed



%client-session-changed

```



Unknown event jangan membuat backend crash.



Log sebagai debug:



```text

unknown tmux control event

```



---



# 30. State Resynchronization



Frontend tidak boleh berasumsi event selalu sempurna.



Backend menyediakan:



```text

FullSnapshot()

```



Dipanggil saat:



- WebSocket connect;

- reconnect;

- sequence gap;

- parser ambiguity;

- external topology changes;

- user klik refresh.



---



# 31. Sequence Number



State message menggunakan:



```text

seq

```



Contoh:



```json

{

  "type": "state.delta",

  "seq": 144

}

```



Jika FE terakhir memiliki:



```text

142

```



tetapi menerima:



```text

144

```



FE mengirim:



```text

state.resync

```



---



# 32. UI Layout



Main window:



```text

┌────────────────────────────────────────────────────┐

│ Title Bar                                 Settings │

├───────────────┬────────────────────────────────────┤

│               │ Window Tabs                        │

│ Sessions      ├────────────────────────────────────┤

│               │                                    │

│ dev           │                                    │

│ ├ editor      │           Terminal Workspace       │

│ │ ├ pane 0    │                                    │

│ │ └ pane 1    │                                    │

│ ├ server      │                                    │

│ └ logs        │                                    │

│               │                                    │

│ production    │                                    │

│               │                                    │

├───────────────┴────────────────────────────────────┤

│ status: tmux connected                 tmux 3.x    │

└────────────────────────────────────────────────────┘

```



---



# 33. Sidebar



Sidebar menggunakan shadcn:



```text

ScrollArea

ContextMenu

DropdownMenu

Tooltip

Button

```



Tree:



```text

Session

  Window

    Pane

```



Setiap item mempunyai context menu.



---



# 34. Window Tabs



Bagian atas workspace:



```text

[ editor ] [ server ] [ logs ] [+]

```



Klik tab:



```text

select-window

```



Right-click:



```text

Rename

Move Left

Move Right

Close

```



---



# 35. Pane Header



Masing-masing pane mempunyai mini header:



```text

● bash          ~/projects/app          %12

```



Display:



- active indicator;

- current command;

- cwd;

- pane ID;

- optional title.



Toolbar muncul hover:



```text

Split

Zoom

...

```



---



# 36. Status Bar



Display:



```text

Tmux Connected

Session: dev

Window: editor

3 panes

tmux 3.x

```



Saat reconnect:



```text

Reconnecting to tmux...

```



---



# 37. Empty State



Jika tidak ada tmux session:



```text

No tmux sessions



Create your first session to get started.



[ Create Session ]

```



Tidak otomatis membuat session diam-diam.



---



# 38. Command Palette



Shortcut GUI opsional:



```text

Ctrl+Shift+P

```



Membuka shadcn Command.



Actions:



```text

New Session

New Window

Split Right

Split Down

Zoom Pane

Rename Window

Kill Pane

Next Layout

```



Tujuannya bukan mengganti satu kumpulan shortcut tmux dengan shortcut aplikasi.



Semua tetap tersedia melalui mouse.



---



# 39. Context Menu



Session:



```text

New Window

Rename

Kill

```



Window:



```text

New Pane Right

New Pane Down

Rename

Layout

Kill

```



Pane:



```text

Split Right

Split Down

Zoom

Swap

Break To Window

Kill

```



---



# 40. Clipboard



Browser:



```text

Ctrl+Shift+C

```



jika ada selection:



```text

copy

```



Paste:



```text

Ctrl+Shift+V

```



mengirim clipboard sebagai terminal input.



Electron memakai browser clipboard API apabila tersedia.



Tidak perlu custom tmux clipboard engine di MVP.



---



# 41. Terminal Links



Gunakan:



```text

@xterm/addon-web-links

```



Web:



link membuka tab baru.



Electron:



link dibuka menggunakan:



```text

shell.openExternal

```



melalui secure preload IPC.



---



# 42. Settings MVP



UI settings disimpan frontend menggunakan:



```text

localStorage

```



Bukan DB backend.



Settings:



```text

Theme

Font family

Font size

Line height

Scrollback lines

Confirm before killing pane

Confirm before killing window

Confirm before killing session

```



Backend config tetap environment/CLI.



---



# 43. Backend Configuration



Environment:



```text

TMUXGUI_HOST

TMUXGUI_PORT

TMUXGUI_TMUX_SOCKET

TMUXGUI_LOG_LEVEL

TMUXGUI_SCROLLBACK_LINES

```



Default:



```text

TMUXGUI_HOST=127.0.0.1

TMUXGUI_PORT=14101

TMUXGUI_SCROLLBACK_LINES=2000

```



Production Electron:



```text

TMUXGUI_PORT=0

```



untuk dynamic port.



---



# 44. Tmux Socket



Secara default gunakan tmux server milik user.



Jangan membuat dedicated server secara default.



Tujuannya agar:



```bash

tmux ls

```



dan GUI melihat state yang sama.



Optional:



```text

TMUXGUI_TMUX_SOCKET

```



untuk custom socket.



Digunakan juga pada integration testing agar tidak mengganggu session pengguna.



---



# 45. Security



Default backend:



```text

127.0.0.1

```



Jangan bind:



```text

0.0.0.0

```



secara default karena API ini mempunyai kemampuan mengirim command ke shell.



CORS tidak diperlukan karena:



- production FE berasal dari backend yang sama;

- Vite development menggunakan proxy.



WebSocket wajib melakukan Origin validation.



Remote network access/authentication dianggap phase berikutnya.



---



# 46. Frontend State Management



Gunakan:



```text

TanStack Query

```



untuk read-only HTTP request seperti:



```text

health

tmux info

session enumeration

```



Gunakan:



```text

Zustand

```



untuk:



```text

active session

active window

active pane

WebSocket state

terminal registry

state snapshot

connection state

```



---



# 47. Error Handling



Contoh:



tmux mati:



```text

Connection to tmux was lost.

Attempting to reconnect...

```



Pane dibunuh dari CLI ketika sedang aktif:



```text

activePane tidak lagi ada

→ pilih active pane dari tmux snapshot

```



Session dibunuh:



```text

WebSocket disconnected

→ refresh sessions

→ tampil empty state atau session lain

```



---



# 48. Reconnect



WebSocket reconnect:



```text

250 ms

500 ms

1 s

2 s

5 s

10 s

```



maksimum:



```text

10 s

```



Setelah reconnect:



```text

request full snapshot

```



Jangan mencoba replay state frontend lama tanpa validasi.



---



# 49. Backend Graceful Shutdown



Backend menangani:



```text

SIGINT

SIGTERM

```



Shutdown:



1. close HTTP server;

2. cancel WebSocket clients;

3. terminate control-mode tmux child processes;

4. wait;

5. exit.



Jangan kill tmux server.



Menutup aplikasi **tidak boleh membunuh session tmux**.



---



# 50. Electron Lifecycle



Electron start:



```text

app ready

↓

spawn backend

↓

backend prints BACKEND_PORT

↓

BrowserWindow.loadURL()

```



Backend stdout:



```text

BACKEND_PORT:49218

```



Electron parse port tersebut.



Electron quit:



```text

kill backend process

```



Tapi tmux session tetap hidup.



---



# 51. Electron Security



BrowserWindow:



```js

nodeIntegration: false

contextIsolation: true

sandbox: true

```



Expose API seminimal mungkin melalui preload.



Contoh:



```text

window.desktop.platform

window.desktop.openExternal()

window.desktop.window.minimize()

window.desktop.window.maximize()

window.desktop.window.close()

```



Tidak expose:



```text

require

fs

child_process

```



ke renderer.



---



# 52. Project Bootstrap — Prerequisites



Linux environment:



```bash

node --version

npm --version

go version

tmux -V

git --version

```



Recommended:



```text

Node.js 22 LTS

Go current stable

tmux >= 3.2

```



---



# 53. Step 1 — Membuat Repository



```bash

mkdir tmux-gui

cd tmux-gui



git init



mkdir be

mkdir desktop

mkdir scripts

mkdir dist



touch README.md

touch .gitignore

touch Makefile

```



---



# 54. Step 2 — Bootstrap Go Backend



```bash

cd be



go mod init github.com/<OWNER>/tmux-gui/be

```



Install dependency WebSocket:



```bash

go get github.com/coder/websocket

```



Rapikan:



```bash

go mod tidy

```



Kemudian:



```bash

cd ..

```



Sebisa mungkin backend menggunakan Go standard library.



HTTP router menggunakan:



```text

net/http

```



Tidak perlu Gin/Echo/Fiber untuk project sekecil ini.



---



# 55. Step 3 — Bootstrap Vite React



Dari root:



```bash

npm create vite@latest fe -- --template react-ts

```



Kemudian:



```bash

cd fe

npm install

```



---



# 56. Step 4 — Install Tailwind



```bash

npm install tailwindcss @tailwindcss/vite

npm install -D @types/node

```



---



# 57. Step 5 — Initialize shadcn



```bash

npx shadcn@latest init

```



Gunakan:



```text

TypeScript

CSS variables

src/components

@/* alias

```



Jangan membuat Button/Card/Dialog sendiri kalau sudah tersedia di shadcn.



---



# 58. Step 6 — Generate shadcn Components



Jalankan command:



```bash

npx shadcn@latest add \

  button \

  card \

  dialog \

  alert-dialog \

  dropdown-menu \

  context-menu \

  command \

  input \

  label \

  scroll-area \

  separator \

  sheet \

  tabs \

  tooltip \

  badge \

  select \

  switch \

  resizable \

  sonner

```



Semua component tersebut harus dihasilkan shadcn CLI.



JANGAN ditulis ulang manual.



---



# 59. Step 7 — Install Frontend Libraries



```bash

npm install \

  @xterm/xterm \

  @xterm/addon-fit \

  @xterm/addon-web-links \

  @tanstack/react-query \

  zustand \

  react-router-dom \

  lucide-react

```



Testing:



```bash

npm install -D \

  vitest \

  jsdom \

  @testing-library/react \

  @testing-library/jest-dom

```



Kemudian:



```bash

cd ..

```



---



# 60. Step 8 — Bootstrap Electron



```bash

cd desktop



npm init -y



npm install -D \

  electron \

  electron-builder \

  concurrently \

  wait-on

```



Tidak menggunakan Electron renderer template karena renderer sudah berada di:



```text

../fe

```



Kemudian:



```bash

mkdir resources

touch resources/.gitkeep



cd ..

```



---



# 61. Step 9 — Configure Vite Output



`fe/vite.config.ts` diubah agar production build langsung masuk:



```text

be/internal/web/dist

```



Konsep:



```ts

build: {

  outDir: "../be/internal/web/dist",

  emptyOutDir: true

}

```



Development:



```text

Vite :14102

```



Proxy:



```text

/api → http://127.0.0.1:14101

```



WebSocket proxy:



```text

ws: true

```



Dengan demikian FE selalu memanggil:



```ts

fetch("/api/...")

```



bukan hardcoded backend URL.



---



# 62. Go Embedded Frontend



Backend menggunakan:



```go

//go:embed all:dist

```



di:



```text

be/internal/web/embed.go

```



Backend menyediakan SPA fallback.



Request:



```text

/

```



serve:



```text

index.html

```



Request:



```text

/assets/index-xxx.js

```



serve asset.



Unknown non-API path:



```text

index.html

```



untuk React routing.



Request:



```text

/api/...

```



tidak boleh terkena SPA fallback.



---



# 63. Development Commands



## Web only



```bash

make dev-web

```



Secara internal menjalankan:



Terminal 1:



```bash

cd be

go run ./cmd/server

```



Terminal 2:



```bash

cd fe

npm run dev -- --port 14102

```



---



# 64. Desktop Development



```bash

make dev-desktop

```



Flow:



```text

Electron

├── spawn Go :14101

└── load Vite :14102

```



Electron development dapat menjalankan Vite dengan:



```text

concurrently

```



---



# 65. Production Build



```bash

make build

```



Equivalent:



```bash

cd fe

npm run build



cd ../be

go build -o ../dist/tmux-gui-server ./cmd/server

```



Karena FE sudah embedded:



```text

dist/tmux-gui-server

```



adalah satu executable web application.



Run:



```bash

./dist/tmux-gui-server

```



Kemudian:



```text

http://127.0.0.1:14101

```



---



# 66. Electron Build



Step:



```bash

cd fe

npm run build

```



Kemudian:



```bash

cd ../be

go build -o ../desktop/resources/tmux-gui-server ./cmd/server

```



Kemudian:



```bash

cd ../desktop

npm run build:linux

```



Electron Builder menghasilkan:



```text

AppImage

.deb

```



Backend binary dimasukkan menggunakan:



```text

extraResources

```



Tidak perlu memasukkan FE dist secara terpisah karena sudah embedded di backend.



---



# 67. Recommended Makefile



Targets:



```text

make install

make dev-web

make dev-desktop

make build-fe

make build-be

make build

make build-desktop

make test

make test-be

make test-fe

make clean

```



---



# 68. Struktur Folder Final



```text

tmux-gui/

│

├── .gitignore

├── README.md

├── Makefile

│

├── scripts/

│   ├── dev-web.sh

│   ├── build.sh

│   ├── build-desktop.sh

│   └── test.sh

│

├── be/

│   ├── go.mod

│   ├── go.sum

│   │

│   ├── cmd/

│   │   └── server/

│   │       └── main.go

│   │

│   └── internal/

│       │

│       ├── config/

│       │   └── config.go

│       │

│       ├── server/

│       │   ├── server.go

│       │   ├── router.go

│       │   └── health.go

│       │

│       ├── web/

│       │   ├── embed.go

│       │   ├── handler.go

│       │   └── dist/

│       │       └── .gitkeep

│       │

│       ├── tmux/

│       │   ├── model.go

│       │   ├── socket.go

│       │   ├── executor.go

│       │   ├── command.go

│       │   ├── snapshot.go

│       │   ├── parser.go

│       │   ├── control.go

│       │   ├── monitor.go

│       │   ├── input_batcher.go

│       │   └── service.go

│       │

│       └── realtime/

│           ├── protocol.go

│           ├── handler.go

│           ├── client.go

│           └── hub.go

│

├── fe/

│   ├── package.json

│   ├── package-lock.json

│   ├── components.json

│   ├── index.html

│   ├── vite.config.ts

│   ├── tsconfig.json

│   ├── tsconfig.app.json

│   ├── tsconfig.node.json

│   │

│   └── src/

│       ├── main.tsx

│       ├── App.tsx

│       ├── index.css

│       │

│       ├── components/

│       │   ├── ui/

│       │   └── layout/

│       │       ├── AppShell.tsx

│       │       ├── AppSidebar.tsx

│       │       ├── AppTitleBar.tsx

│       │       └── StatusBar.tsx

│       │

│       ├── features/

│       │   ├── sessions/

│       │   │   ├── SessionTree.tsx

│       │   │   ├── SessionItem.tsx

│       │   │   ├── CreateSessionDialog.tsx

│       │   │   └── SessionContextMenu.tsx

│       │   │

│       │   ├── windows/

│       │   │   ├── WindowTabs.tsx

│       │   │   ├── WindowToolbar.tsx

│       │   │   ├── WindowContextMenu.tsx

│       │   │   └── LayoutSelector.tsx

│       │   │

│       │   ├── panes/

│       │   │   ├── PaneWorkspace.tsx

│       │   │   ├── PaneView.tsx

│       │   │   ├── PaneHeader.tsx

│       │   │   ├── PaneContextMenu.tsx

│       │   │   └── PaneResizeHandle.tsx

│       │   │

│       │   ├── terminal/

│       │   │   ├── TerminalView.tsx

│       │   │   ├── useTerminal.ts

│       │   │   └── terminalRegistry.ts

│       │   │

│       │   └── settings/

│       │       ├── SettingsDialog.tsx

│       │       └── TerminalSettings.tsx

│       │

│       ├── hooks/

│       │   ├── useTmuxSocket.ts

│       │   └── useKeyboardShortcut.ts

│       │

│       ├── stores/

│       │   ├── appStore.ts

│       │   ├── tmuxStore.ts

│       │   └── settingsStore.ts

│       │

│       ├── lib/

│       │   ├── api.ts

│       │   ├── websocket.ts

│       │   ├── protocol.ts

│       │   ├── tmux-types.ts

│       │   └── utils.ts

│       │

│       └── types/

│           └── desktop.d.ts

│

└── desktop/

    ├── package.json

    ├── package-lock.json

    ├── main.js

    ├── preload.js

    └── resources/

        └── .gitkeep

```



---



# 69. File Plan — Root



| File | Action | Fungsi |

|---|---|---|

| `.gitignore` | ADD | Ignore dist/node_modules/generated FE |

| `README.md` | ADD | Setup dan usage |

| `Makefile` | ADD | Shortcut development/build/test |

| `scripts/dev-web.sh` | ADD | Start BE + FE |

| `scripts/build.sh` | ADD | Build FE lalu Go |

| `scripts/build-desktop.sh` | ADD | Build backend + Electron |

| `scripts/test.sh` | ADD | Run seluruh test |



---



# 70. File Plan — Backend



| File | Action | Fungsi |

|---|---|---|

| `be/go.mod` | ADD via `go mod init` | Go module |

| `be/go.sum` | GENERATED | Dependency lock |

| `be/cmd/server/main.go` | ADD | Program entrypoint |

| `internal/config/config.go` | ADD | Env/flag config |

| `internal/server/server.go` | ADD | HTTP server lifecycle |

| `internal/server/router.go` | ADD | REST/WS/static routing |

| `internal/server/health.go` | ADD | Health/tmux info endpoint |

| `internal/web/embed.go` | ADD | `go:embed` FE |

| `internal/web/handler.go` | ADD | Static + SPA fallback |

| `internal/web/dist/.gitkeep` | ADD | Embed directory placeholder |

| `internal/tmux/model.go` | ADD | Session/window/pane structs |

| `internal/tmux/socket.go` | ADD | Resolve tmux socket |

| `internal/tmux/executor.go` | ADD | Safe one-shot read commands |

| `internal/tmux/command.go` | ADD | Typed mutating commands |

| `internal/tmux/snapshot.go` | ADD | Full tmux state query |

| `internal/tmux/parser.go` | ADD | Parse control-mode output |

| `internal/tmux/control.go` | ADD | Child process stdin/stdout |

| `internal/tmux/monitor.go` | ADD | Event loop |

| `internal/tmux/input_batcher.go` | ADD | Batch terminal input |

| `internal/tmux/service.go` | ADD | High-level tmux operations |

| `internal/realtime/protocol.go` | ADD | WS message structs |

| `internal/realtime/handler.go` | ADD | WebSocket handler |

| `internal/realtime/client.go` | ADD | Per-connection state |

| `internal/realtime/hub.go` | ADD | Event broadcasting |



Tidak ada file backend yang perlu DELETE karena project baru.



---



# 71. File Plan — Frontend Generated



Dibuat oleh:



```bash

npm create vite@latest

```



Kemudian shadcn CLI.



### EDIT



```text

fe/package.json

fe/index.html

fe/vite.config.ts

fe/tsconfig.json

fe/tsconfig.app.json

fe/src/main.tsx

fe/src/App.tsx

fe/src/index.css

```



### DELETE boilerplate Vite



```text

fe/src/App.css

fe/src/assets/react.svg

fe/public/vite.svg

```



jika terdapat pada template yang dipakai.



### GENERATED BY SHADCN



```text

fe/components.json

fe/src/components/ui/*

fe/src/lib/utils.ts

```



Jangan dibuat ulang manual.



---



# 72. File Plan — Frontend Application



Semua berikut ADD:



```text

components/layout/AppShell.tsx

components/layout/AppSidebar.tsx

components/layout/AppTitleBar.tsx

components/layout/StatusBar.tsx

```



Sessions:



```text

features/sessions/SessionTree.tsx

features/sessions/SessionItem.tsx

features/sessions/CreateSessionDialog.tsx

features/sessions/SessionContextMenu.tsx

```



Windows:



```text

features/windows/WindowTabs.tsx

features/windows/WindowToolbar.tsx

features/windows/WindowContextMenu.tsx

features/windows/LayoutSelector.tsx

```



Panes:



```text

features/panes/PaneWorkspace.tsx

features/panes/PaneView.tsx

features/panes/PaneHeader.tsx

features/panes/PaneContextMenu.tsx

features/panes/PaneResizeHandle.tsx

```



Terminal:



```text

features/terminal/TerminalView.tsx

features/terminal/useTerminal.ts

features/terminal/terminalRegistry.ts

```



Settings:



```text

features/settings/SettingsDialog.tsx

features/settings/TerminalSettings.tsx

```



State/network:



```text

stores/appStore.ts

stores/tmuxStore.ts

stores/settingsStore.ts



lib/api.ts

lib/websocket.ts

lib/protocol.ts

lib/tmux-types.ts

```



---



# 73. File Plan — Desktop



### Generated using command



```text

desktop/package.json

desktop/package-lock.json

```



### ADD manually



```text

desktop/main.js

desktop/preload.js

desktop/resources/.gitkeep

```



### EDIT



```text

desktop/package.json

```



untuk scripts dan electron-builder config.



Tidak membuat renderer Electron terpisah.



---



# 74. Backend Unit Tests



Tambahkan:



```text

internal/tmux/parser_test.go

internal/tmux/command_test.go

internal/tmux/snapshot_test.go

internal/tmux/input_batcher_test.go

```



Test:



- stable ID targeting;

- parsing `%output`;

- parsing layout changes;

- invalid event;

- command escaping;

- UTF-8 input;

- control sequence input;

- malformed tmux output.



---



# 75. Backend Integration Test



Gunakan tmux socket khusus.



Contoh:



```bash

tmux -L tmux-gui-test kill-server || true

tmux -L tmux-gui-test new-session -d -s test

```



Environment test:



```bash

TMUXGUI_TMUX_SOCKET=tmux-gui-test

```



Test jangan pernah memakai tmux server user.



Scenario:



1. create session;

2. detect session;

3. split pane;

4. detect pane;

5. rename window;

6. resize;

7. kill pane;

8. kill session.



Cleanup:



```bash

tmux -L tmux-gui-test kill-server

```



---



# 76. Frontend Tests



Vitest tests:



```text

tmuxStore.test.ts

PaneWorkspace.test.tsx

protocol.test.ts

```



Test utama:



- merge snapshot;

- sequence gap;

- active pane removed;

- pane geometry calculation;

- reconnect state;

- command error display.



---



# 77. Acceptance Criteria — Architecture



Feature dianggap selesai jika:



- Go backend dapat berjalan tanpa Electron.

- FE dapat dibuka dari browser.

- production Go binary serve FE.

- Electron menggunakan backend yang sama.

- tidak ada Node backend.

- tidak ada duplicate terminal backend.

- tmux tetap source of truth.

- tidak ada database session/window/pane.



---



# 78. Acceptance Criteria — Session



- existing tmux sessions terlihat.

- session CLI baru muncul tanpa restart.

- create session GUI muncul di `tmux ls`.

- rename GUI terlihat di CLI.

- rename CLI terlihat di GUI.

- kill session GUI membunuh tmux session asli.

- app close tidak membunuh session.



---



# 79. Acceptance Criteria — Window



- existing windows muncul.

- window CLI baru muncul.

- create window GUI benar-benar membuat tmux window.

- rename sinkron dua arah.

- kill sinkron dua arah.

- pindah window bekerja.

- layout preset bekerja.



---



# 80. Acceptance Criteria — Pane



- pane layout sesuai tmux.

- split GUI membuat pane asli.

- split CLI muncul di GUI.

- kill sinkron.

- resize mouse mengubah ukuran tmux.

- zoom bekerja.

- swap bekerja.

- break pane bekerja.

- stable `%pane_id` digunakan untuk command.



---



# 81. Acceptance Criteria — Terminal



Harus bekerja untuk:



```text

bash

zsh

fish

vim/neovim

nano

htop

less

git

npm

go

python

```



Minimum behavior:



- output muncul realtime;

- input keyboard bekerja;

- arrow keys bekerja;

- Ctrl+C bekerja;

- Ctrl+D bekerja;

- Ctrl+Z bekerja;

- Tab bekerja;

- UTF-8 bekerja;

- resize bekerja;

- scrollback tersedia;

- copy/paste bekerja.



---



# 82. Acceptance Criteria — External CLI



Scenario wajib:



Terminal luar:



```bash

tmux new -s external

```



GUI:



```text

external muncul

```



Kemudian:



```bash

tmux split-window

```



GUI:



```text

pane baru muncul

```



Kemudian:



```bash

tmux rename-window api

```



GUI:



```text

nama berubah menjadi api

```



Kemudian GUI:



```text

Split Right

```



CLI:



```bash

tmux list-panes

```



harus memperlihatkan pane baru.



---



# 83. Implementation Phase 1 — Bootstrap



Lakukan:



```text

repo

Go

Vite

shadcn

Electron

Makefile

```



Pastikan:



```bash

make dev-web

```



menampilkan halaman React.



Kemudian:



```bash

make build

./dist/tmux-gui-server

```



menampilkan React dari Go.



Jangan mengimplementasikan tmux dahulu sebelum pipeline ini berfungsi.



---



# 84. Implementation Phase 2 — Read-only Tmux State



Implement:



```text

tmux detection

version

sessions

windows

panes

geometry

```



UI:



```text

sidebar

window tabs

pane placeholders

```



Belum terminal.



Acceptance:



state CLI terlihat GUI.



---



# 85. Implementation Phase 3 — Control Mode



Implement:



```text

tmux -CC

parser

monitor

WebSocket

reconnect

snapshot

delta

```



Acceptance:



perubahan CLI realtime mengubah GUI.



---



# 86. Implementation Phase 4 — Terminal



Implement:



```text

xterm.js

capture-pane

%output

terminal input

input batching

resize

```



Acceptance:



interactive shell usable.



Ini milestone paling penting.



---



# 87. Implementation Phase 5 — GUI Tmux Actions



Implement:



```text

create/rename/kill session



create/rename/kill/select window



split/kill/select/zoom pane



layout



resize drag

```



Setiap command harus menggunakan typed Go method.



Jangan mengizinkan frontend mengirim arbitrary:



```text

tmux command string

```



ke backend.



---



# 88. Implementation Phase 6 — UX



Implement:



```text

context menus

dialogs

command palette

confirm kill

terminal settings

status bar

reconnect indicator

empty state

error state

```



---



# 89. Implementation Phase 7 — Electron



Setelah web version stabil:



```text

Electron process lifecycle

backend spawn

dynamic port

native window

packaging

AppImage

deb

```



Jangan mengembangkan Electron-specific tmux implementation.



Electron tetap shell.



---



# 90. Implementation Phase 8 — Testing & Hardening



Test:



```text

external CLI sync

large terminal output

rapid typing

vim

htop

pane drag

window deletion

session deletion

backend restart

tmux restart

Electron restart

```



Stress:



```text

10+ windows

20+ panes

large scrollback

continuous logs

```



---



# 91. Non Goals V1



Tidak dikerjakan:



```text

SSH remote server

Windows

macOS

custom shell implementation

replacement terminal emulator backend

file manager/SFTP

collaboration

multi-user accounts

cloud sync

tmux config editor

plugin marketplace

session database

recording/replay terminal

```



---



# 92. Architectural Rules



Rule 1:



```text

tmux adalah source of truth.

```



Rule 2:



```text

FE tidak pernah menjalankan tmux.

```



Rule 3:



```text

Electron tidak pernah menjalankan command tmux langsung.

```



Rule 4:



```text

Semua command tmux melewati Go.

```



Rule 5:



```text

Mutating command menggunakan control-mode path.

```



Rule 6:



```text

Stable IDs digunakan untuk window dan pane.

```



Rule 7:



```text

Frontend production diserve Go.

```



Rule 8:



```text

Tidak ada frontend production server terpisah.

```



Rule 9:



```text

Tidak ada DB untuk menduplikasi state tmux.

```



Rule 10:



```text

Jangan membuat ulang component yang dapat digenerate oleh Vite/shadcn CLI.

```



---



# 93. Definition of Done



Project V1 dinyatakan selesai ketika user dapat:



1. install aplikasi di Linux;

2. membuka Electron tanpa membuka terminal;

3. melihat seluruh session tmux existing;

4. membuka session;

5. melihat windows;

6. melihat layout panes;

7. menggunakan shell secara penuh;

8. membuat session dengan mouse;

9. membuat window dengan mouse;

10. split pane dengan mouse;

11. resize pane dengan drag;

12. rename session/window;

13. berpindah window/pane;

14. zoom pane;

15. kill pane/window/session;

16. mengganti layout;

17. menggunakan tmux CLI di terminal luar;

18. melihat perubahan CLI tersebut langsung di GUI;

19. menutup GUI tanpa menghentikan tmux;

20. membuka GUI kembali dan melanjutkan session yang sama.



Dengan kata lain:



```text

Tmux GUI bukan terminal app yang kebetulan memakai tmux.



Tmux GUI adalah graphical control surface untuk tmux asli.

```

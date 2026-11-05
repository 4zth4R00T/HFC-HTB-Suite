```
██╗  ██╗███████╗ ██████╗
██║  ██║██╔════╝██╔════╝
███████║█████╗  ██║
██╔══██║██╔══╝  ██║
██║  ██║██║     ╚██████╗
╚═╝  ╚═╝╚═╝      ╚═════╝

██╗  ██╗████████╗██████╗   ███████╗██╗   ██╗██╗████████╗███████╗
██║  ██║╚══██╔══╝██╔══██╗  ██╔════╝██║   ██║██║╚══██╔══╝██╔════╝
███████║   ██║   ██████╔╝  ███████╗██║   ██║██║   ██║   █████╗
██╔══██║   ██║   ██╔══██╗  ╚════██║██║   ██║██║   ██║   ██╔══╝
██║  ██║   ██║   ██████╔╝  ███████║╚██████╔╝██║   ██║   ███████╗
╚═╝  ╚═╝   ╚═╝   ╚═════╝   ╚══════╝ ╚═════╝ ╚═╝   ╚═╝   ╚══════╝
```

# HTB Suite

Cliente de terminal para Hack The Box Labs. Todo lo que se hace en la web con
máquinas, retos y VPN, hecho desde la consola y sin abrir el navegador.

Escrito por **Aztharot**.
Pertenece a Aztharot y a **HFC — Hackers Fight Club**.

Sígueme en **instagram.com/its_aztharot** y al club en
**instagram.com/hackersfightclub**.

Gracias a **Lorne**, **Ookami** y **Pablocro** por el apoyo y por la inspiración
que llevó a hacer esto.

---

## Qué hace

| Área | Lo que puedes hacer |
|---|---|
| Máquinas | Listar activas, retiradas y sin publicar, buscar por nombre, filtrar por dificultad, sistema y completadas, ordenar por valoración, fecha, dificultad votada, owns o nombre, marcar favoritas y ver la ficha completa con sinopsis, etiquetas, matriz y bloods |
| Máquina activa | Ver la IP en cuanto HTB la asigna sin salir de la ficha, caducidad, lanzar, parar, reiniciar y extender el tiempo |
| Comunidad | Opiniones, actividad reciente, registro de cambios, writeup oficial, vídeo y guías de la comunidad de cada máquina |
| Búsqueda global | Máquinas, retos, usuarios y equipos desde una sola caja, y el perfil público de cualquier usuario |
| Dashboard | En progreso, recomendadas y favoritas, igual que la portada de la web |
| Challenges | Listar por categoría, filtrar los que te faltan, descargar archivos, **levantar y apagar el contenedor** y enviar la flag |
| Fortress | Listar con las flags que llevas, ver IP y puntos, enviar flags |
| Sherlocks | Listar con búsqueda, estado, favoritos, dificultad, categoría y orden, leer el escenario, responder **tarea por tarea**, bajar el material y **levantar la VM** con sus credenciales |
| Pro Labs | Listar, ver versión, progreso, puntos de entrada, máquinas y flags |
| Tracks | Listar y ver el contenido de cada ruta |
| Seasons | Temporada activa con cuenta atrás, tier, flags, máquinas de la season, leaderboard con tu puesto y recompensas |
| Rankings | Hall of Fame de usuarios, equipos, universidades y países |
| Pwnbox | Estado de la instancia y minutos usados del plan |
| VPN | Listar servidores por producto, cambiar el servidor asignado y bajar el `.ovpn` |
| Perfil | Rango, puntos, owns, bloods, respects y progreso de la season |
| Estadísticas | Barras de progreso por dificultad y categoría, racha de la season y ranking global |
| Diagnóstico | Estado y latencia de cada endpoint que usa la herramienta |

Idiomas: **español**, **inglés** y **ruso**. Se cambia desde Ajustes y se guarda.

Tema oscuro y claro con la misma paleta. En la interfaz Rust se cambia con `F2`.

Cada cosa se trata como lo que es. Los retos pueden traer un zip, un contenedor
que se levanta y se apaga, o las dos cosas. Las fortress son flags contra una IP
fija. Los sherlocks son preguntas que se responden una a una con el formato que
la propia API indica, y según el sherlock traen un zip de evidencias o una VM
Windows a la que entras por RDP con las credenciales que devuelve HTB.

---

## Tres implementaciones

Las tres hablan con la misma API, comparten la carpeta de configuración y
tienen las mismas funciones.

### Rust (recomendada)

Interfaz ratatui completa, con ratón, scrollbar y pantallas a color.

```sh
cd rust/htb-cli
cargo build --release
./target/release/htb
```

Para tenerla a mano:

```sh
sudo install -m755 target/release/htb /usr/local/bin/htb
```

También sirve como CLI directa, sin interfaz:

```sh
htb profile
htb machines list --state active --difficulty easy
htb machines list --favorites --sort rating
htb machines list --state unreleased
htb machines info 981
htb machines tags 981
htb machines reviews 981
htb machines activity 981
htb machines changelog 981
htb machines walkthroughs 981
htb machines favorite 981
htb machines search lame
htb machines dashboard recommended
htb machines spawn 981
htb machines flag 981 'HTB{...}' --difficulty 7
htb challenges list --category Pwn --unsolved
htb challenges list --state retired --sort rating
htb challenges favorite 106
htb challenges reviews 106
htb challenges download 106
htb prolabs --id 25
htb fortress --id 1 --flag 'HTB{...}'
htb sherlocks --page 2
htb sherlocks --search brutus --category dfir
htb sherlocks --download 631
htb tracks --id 61
htb seasons
htb seasons --leaderboard --page 2
htb seasons --rewards
htb rankings teams
htb pwnbox
htb vpn list labs
htb vpn download 253 --product labs --tcp
htb credits
htb token --set
```

### Bash

```sh
./htb.sh
```

Necesita `bash` 4 o superior, `curl`, `jq`, `openssl`, `python3`, `xxd` y
`base64`. Los cuatro últimos son para el vault del token. Si falta alguno lo
dice y para: no instala nada por su cuenta, y menos con `sudo`.

### PowerShell

```sh
pwsh -File ./htb.ps1
```

Necesita PowerShell 7 o superior. Funciona igual en Linux, macOS y Windows.

---

## Cómo moverse

Mismo vocabulario de teclas en las tres versiones, siempre en el mismo sitio y
con el mismo significado.

| Tecla | Qué hace |
|---|---|
| `↑` `↓` | Mueven la selección (también `k` y `j`) |
| `↵` | Abre lo que tengas marcado (también `→`) |
| `←` | Vuelve atrás (también `Esc` y `q`) |
| `AvPág` `RePág` | Saltan de página (también la barra espaciadora) |
| `Inicio` `Fin` | Primera y última fila (también `g` y `G`) |
| Rueda del ratón | Hace scroll por la lista |
| Clic | El primero marca la fila, el segundo la abre |
| Clic derecho | Vuelve atrás |
| `c` | Copia al portapapeles lo que la pantalla ofrezca |
| `?` | Ayuda de navegación |

Las letras de la barra inferior actúan sobre la fila marcada, con la palabra al
lado para que no haya que adivinar: `s lanzar`, `t parar`, `r reiniciar`,
`e extender`, `f flag`, `n siguiente página`, `p página anterior`, `c copiar`.

## Filtros

Ningún filtro se escribe a mano: cada uno abre una lista de la que se elige con
las flechas o el ratón, y la primera opción siempre es «sin filtro».

| Pantalla | Tecla | Filtra por |
|---|---|---|
| Máquinas | `d` | Dificultad: Very Easy, Easy, Medium, Hard, Insane |
| Máquinas | `o` | Sistema: Linux, Windows, FreeBSD, OpenBSD, Android, Other |
| Máquinas | `v` | Estado: resueltas o sin resolver |
| Máquinas | `a` | Orden: valoración, fecha, dificultad votada, user owns, system owns o nombre; `A` lo invierte |
| Máquinas | `m` | Marca o quita la máquina de favoritas |
| Máquinas | `x` | Quita los filtros de golpe |
| Challenges | `c` | Categoría: Web, Crypto, Pwn, Reversing, Forensics… |
| Challenges | `d` | Dificultad |
| Challenges | `u` | Alterna «solo sin resolver» |
| Challenges | `x` | Quita todos |

Los filtros se combinan y la cabecera va diciendo cuáles están puestos, por
ejemplo `Challenges · categoría: Web · dificultad: Very Easy · solo sin
resolver: sí`. Dificultad y sistema de las máquinas viajan a la API como
`difficulty[]` y `os[]`; el resto se aplica sobre lo ya descargado.

La lista de challenges trae el catálogo entero (unas 850 entradas, en páginas de
cien) para que un filtro de categoría devuelva todo lo que existe y no solo lo
que esté activo ese día.

En la lista de challenges `c` abre el filtro de categoría, porque ahí no hay
ninguna dirección que copiar; en las pantallas de detalle `c` copia. La barra
inferior siempre dice cuál de las dos toca.

Lo que copia cada pantalla:

| Pantalla | Qué acaba en el portapapeles |
|---|---|
| Máquinas y Mi objetivo | La IP de la máquina que tengas corriendo |
| Challenge con entorno | `ip:puerto` y debajo `http://ip:puerto`, listo para el navegador |
| Fortress | La IP de la fortaleza |
| Sherlock con VM | La IP y debajo la orden `xfreerdp` ya montada |

Los zip de los challenges van cifrados con `hackthebox` y los de los sherlocks
con `hacktheblue`; cuando la API trae una contraseña propia se usa esa y la
pantalla la enseña.

---

## Starting Point

El menú de máquinas lleva a Starting Point, con los tres niveles y una barra del
progreso de cada uno. Al entrar en un nivel salen sus máquinas con el sistema, la
dificultad y si ya las tienes. Desde ahí se lanzan, se paran, se copia la IP y se
envía la flag como en cualquier otra máquina.

Ojo con la VPN: las máquinas de Starting Point viven en una red distinta a la de
labs. Si la máquina responde al ping pero todos los puertos salen filtrados, es
que sigues conectado a la VPN equivocada; cambia el producto a `starting_point`
en la pantalla de VPN y vuelve a levantar el túnel.

## Pasos guiados

Muchas máquinas traen un modo guiado: una serie de preguntas cortas que te llevan
de la mano desde el escaneo hasta la flag. **No hay que pulsar nada para verlas**:
las preguntas son la propia pantalla de la máquina, tanto al abrir su detalle como
en «Mi objetivo» mientras la tienes corriendo. Arriba quedan los datos de la
máquina y debajo la lista de preguntas.

Cada fila muestra la pregunta y el formato que espera la respuesta: los asteriscos
indican cuántos caracteres tiene, así que `*****.***` es un dominio de cinco letras
más extensión de tres. `↵` sobre una pregunta la responde; las ya contestadas
enseñan en su lugar la respuesta que diste.

La última entrada de la lista es siempre el envío de la flag real. Las máquinas sin
modo guiado enseñan directamente sus casillas de flag user y root, y las listas
llevan una barra con el porcentaje de preguntas respondidas para saber por dónde
vas sin entrar.

## Valorar la dificultad

Al enviar una flag, HTB pide que puntúes la máquina o el reto. Ya no hay que
escribir un número a ciegas: sale la escala con sus nombres, de «Pan comido» a
«Brainfuck». El detalle de cada máquina muestra además la media de lo que ha
votado la gente y el desglose completo en barras.

Al pedir una flag, si el portapapeles ya trae algo con forma de flag se ofrece
como valor por omisión: basta pulsar `↵`.

Diferencias por versión:

| Tecla | Versión | Qué hace |
|---|---|---|
| `Ctrl+C` | Rust | Copia |
| `Ctrl+V` | Rust | Pega en los campos de texto |
| `Ctrl+Q` | Rust | Cierra el programa |
| `Ctrl+C` | Bash y PowerShell | Cierra el programa |
| `F2` | Rust | Cambia entre tema oscuro y claro |
| `F5` o `Ctrl+R` | Rust | Vuelve a pedir los datos a la API |
| `Ctrl+U` `Ctrl+W` | Rust | Borran la línea o la última palabra al escribir |

---

## Configuración

| Archivo | Para qué |
|---|---|
| `~/.config/htb-hfc/token.vault` | Tu App Token, cifrado, con permisos `600` |
| `~/.config/htb-hfc/lang` | `es`, `en` o `ru` |
| `~/.config/htb-hfc/theme` | `dark` o `light` |
| `~/.config/htb-hfc/vpn/current.ovpn` | Copia del último `.ovpn` descargado |
| `~/HTB/` | Descargas de retos, sherlocks y VPN |

El token se genera en `app.hackthebox.com` → Profile → Settings → Create App
Token. La primera vez el programa lo pide, pide también una passphrase y guarda
el token cifrado con ella. A partir de ahí la passphrase se pide en cada
arranque; sin ella el fichero no se abre, y no hay forma de recuperarla.

| Variable | Efecto |
|---|---|
| `HTB_TOKEN` | Usa ese token en vez del vault. Para scripts: queda visible en el entorno del proceso |
| `HTB_DL_DIR` | Cambia la carpeta de descargas |
| `HTB_VPN_DIR` | Cambia la carpeta de la VPN |
| `HTB_NO_ANIM=1` | Salta la animación de arranque |
| `HTB_ALLOW_ROOT=1` | Permite ejecutar como root, que por defecto se rechaza |

De tu cuenta solo se muestra el nick. El nombre real nunca aparece en pantalla,
y el plan solo sale cuando la API rechaza algo por no ser de pago.

---

## Cómo se guarda el token

El token es una credencial con acceso completo a tu cuenta de HTB. Antes se
escribía en un fichero de texto; ahora vive dentro de un contenedor cifrado.

**El contenedor.** AES-256-CBC para el contenido y HMAC-SHA256 sobre todo lo
demás, en ese orden (Encrypt-then-MAC): la etiqueta se comprueba *antes* de
descifrar nada, así que un fichero manipulado se rechaza sin llegar al
descifrador. Las dos claves y el IV salen de 80 bytes derivados de tu
passphrase, con salt nuevo en cada escritura. El texto en claro se rellena
hasta un múltiplo de 64 bytes para que el tamaño del fichero no diga cuánto
mide el token.

**La derivación.** El cliente de Rust usa **Argon2id** con 128 MiB, 3 pasadas y
4 carriles: el coste en memoria es lo que le quita la ventaja a un atacante con
GPU. Las versiones de Bash y PowerShell usan **PBKDF2-HMAC-SHA512 con 600 000
rondas**, muy por encima de las 210 000 que recomienda OWASP. No usan Argon2id
porque ninguna de las dos puede ejecutarlo sin pasarle la passphrase a otro
programa por la línea de órdenes, y `/proc/<pid>/cmdline` lo lee cualquier
usuario de la máquina: un KDF algo más flojo dentro del proceso protege más que
uno excelente cuyo secreto se anuncia.

**Las tres versiones comparten formato.** Un vault escrito desde `htb.sh` se
abre desde `htb.ps1` y desde el binario de Rust, y al revés. La única asimetría
es Argon2id, que solo entiende Rust; si vas a alternar entre las tres, crea el
vault con `htb token --set --portable` y las tres lo abrirán. Hay tests que
comprueban esta compatibilidad con vaults reales generados por las otras dos
implementaciones (`cargo test` en `rust/htb-cli/`).

| Orden | Qué hace |
|---|---|
| `htb token` | Dice si hay vault y con qué está cifrado |
| `htb token --set` | Pide un token nuevo por teclado y lo cifra |
| `htb token --set --portable` | Igual, pero con PBKDF2 para compartirlo con Bash y PowerShell |
| `htb token --passphrase` | Cambia la passphrase sin tocar el token ni el KDF |
| `htb token --forget` | Sobrescribe el vault y lo borra |

En las versiones de Bash y PowerShell lo mismo está en Ajustes.

---

## Qué se hizo para que no se escape nada

La suite maneja un token, flags e IPs de máquinas, y se ejecuta en la misma
sesión donde estás trabajando. Estas son las vías por las que eso podía salir
fuera y cómo se cerraron:

- **La secuencia OSC 52 al copiar.** Las tres versiones le pedían al terminal
  que copiara el texto con `\033]52;c;<base64>`. Sobre SSH, `tmux` o `screen`
  eso manda la flag o la IP a la máquina que dibuja el terminal, que no tiene
  por qué ser esta. Se quitó: si no hay portapapeles local, se dice que no se
  pudo copiar.
- **El token en la línea de órdenes.** `curl -H "Authorization: Bearer …"` deja
  el token en `/proc/<pid>/cmdline`, que es legible por cualquier usuario. Ahora
  la cabecera entra por `curl --config -`, por la entrada estándar. Por lo mismo,
  `htb token --set` pide el token por teclado en vez de aceptarlo como argumento,
  donde además quedaría en el historial del shell.
- **`sudo apt-get install` automático.** La versión de Bash instalaba sus
  dependencias sola llamando al gestor de paquetes con privilegios. Ahora solo
  dice qué falta.
- **Ejecutar como root.** Las tres se niegan, salvo `HTB_ALLOW_ROOT=1`. Nada de
  lo que hacen lo necesita, y hecho como root deja ficheros de root en el `HOME`
  de un usuario normal, que es el patrón con el que se escala.
- **El `PATH` heredado.** La suite llama a programas por su nombre (`xclip`,
  `wl-copy`, `openssl`). Se descartan del `PATH` los directorios relativos y los
  que puede reescribir un tercero, para que ese nombre no lo resuelva quien haya
  llegado antes.
- **Permisos y sitio de los ficheros.** `umask 077`; el directorio de
  configuración se fuerza a `700` y se rechaza si es un enlace simbólico o si
  pertenece a otro usuario; los `.ovpn`, que llevan dentro la clave privada de
  la VPN, se escriben `600`; los temporales salieron de `/tmp` al directorio de
  configuración; y el texto que se copia al portapapeles ya no pasa por un
  fichero intermedio en PowerShell, sino por la entrada estándar del proceso.
- **El binario de Rust, en concreto.** El perfil de release va sin símbolos, sin
  información de depuración y con `panic = "abort"`; un fallo imprime una línea
  genérica en vez del fichero, la línea y el valor que lo provocó, y
  `RUST_BACKTRACE` se ignora. La batería de comprobaciones internas
  (`--self-test`) ya no existe fuera de las compilaciones de desarrollo. El
  proceso arranca con los volcados de core desactivados y, en Linux, marcado
  como no depurable (`PR_SET_DUMPABLE`), para que ni un `ptrace` de otro proceso
  tuyo ni un core dump en disco lleguen al token. El token en memoria vive en
  `Zeroizing`, y el cliente HTTP solo habla `https`.
- **Los restos de antes.** Al arrancar, si encuentra un token en claro de una
  versión anterior, lo cifra y sobrescribe el fichero original antes de
  borrarlo.

---

## Notas sobre la API de HTB

Cosas que fueron cambiando y que la herramienta ya tiene en cuenta:

- Los productos de VPN válidos hoy son `labs`, `starting_point`, `sherlocks`,
  `fortresses`, `release_arena` y `competitive`. `endgame` y `pro_labs` devuelven
  422. Los sherlocks con VM viven en su propia VPN, no en la de labs.
- La API solo entrega el `.ovpn` del servidor que tengas asignado, así que al
  pedir otro la herramienta te cambia primero (avisando) y luego descarga.
- La dificultad al enviar flags va del 1 al 10 en máquinas, pero en retos la
  API exige múltiplos de diez. La herramienta lo traduce por dentro.
- Los sherlocks no tienen una flag única: cada tarea se responde por separado
  contra `/sherlocks/{id}/tasks/{task}/flag`, y la propia API dice qué formato
  espera. La contraseña del zip puede venir en `file_password`; si no viene, es
  `hackthebox`.
- No todos los sherlocks traen zip. `play_methods` dice si es `download` o `vm`.
  Los de tipo `vm` se levantan con `POST /challenge/start/vm/{id}` (sí, por la
  ruta de challenges), devuelven usuario y contraseña de RDP, se apagan con
  `/vm/terminate` pasando el id del sherlock y se reinician con
  `/challenge/reset/vm/{id}`.
- Los contenedores de los retos se levantan con
  `POST /container/start {containerable_id}` y se apagan con `/container/stop`.
  La IP y los puertos aparecen luego en `play_info`.
- `GET /virtual_machine/active` (v5) dice qué tienes corriendo sea máquina,
  sherlock o reto; `/machine/active` solo ve máquinas.
- `Starting Point` ya no tiene ruta pública: `/sp/tiers` responde 404 en todas
  sus variantes.
- Las máquinas de Release Arena no arrancan si estás en un servidor normal.
- Tras lanzar una máquina, HTB tarda un rato en asignar la IP. La herramienta
  espera y sondea en vez de mostrar un guion.
- Cloudflare corta con un 1015 si se pide demasiado seguido. Cuando pasa, la
  herramienta lo dice con claridad en vez de culpar al token.

---

## Estructura

```
htb.sh                      lanzador de la versión Bash
htb.ps1                     lanzador de la versión PowerShell
lib/
  htb-core.sh               config, tema, HTTP, lista navegable, lectura de teclas
  htb-vault.sh              cifrado del token: AES-256-CBC + HMAC sobre PBKDF2
  htb-i18n.sh               textos en español, inglés y ruso
  htb-brand.sh              marca HFC, firma y arranque
  htb-machines.sh           máquinas y acciones sobre la activa
  htb-catalog.sh            challenges, prolabs, fortress, sherlocks, tracks, seasons
  htb-vpn.sh                servidores y descarga del .ovpn
  htb-profile.sh            perfil, estadísticas, diagnóstico, ajustes y créditos
  Htb.Vault.ps1             el mismo vault en .NET puro
  Htb.*.ps1                 los mismos módulos para PowerShell
rust/htb-cli/
  src/main.rs               subcomandos y arranque
  src/vault.rs              cifrado del token: AES-256-CBC + HMAC sobre Argon2id
  src/session.rs            apertura del vault, alta y migración del token
  src/hardening.rs          lo que cierra el proceso antes de tocar nada
  src/http.rs               cliente de la API y lectura de respuestas
  src/brand.rs              marca HFC y firma
  src/tui/
    event.rs                teclado, ratón y pegado
    theme.rs                paleta oscura y clara
    i18n.rs                 textos en los tres idiomas
    widgets/                lista navegable y campo de texto
    screens/                una pantalla por sección
  src/commands/             las mismas funciones sin interfaz
```

---

## Aviso

Esta herramienta habla con tu propia cuenta usando tu propio token. No hace
nada que no puedas hacer desde la web de HTB. El token vive en tu disco cifrado
con una passphrase que solo conoces tú, en un fichero con permisos `600`, y no
sale de tu máquina más que hacia `labs.hackthebox.com` por TLS.

Si pierdes la passphrase no hay recuperación posible: borra el vault con
`htb token --forget`, genera un App Token nuevo en HTB y vuelve a empezar. Y si
crees que el token se ha visto comprometido, revócalo en
`app.hackthebox.com` → Profile → Settings; cambiar la passphrase protege el
fichero, no el token que ya estaba fuera.

---

Aztharot · instagram.com/its_aztharot
HFC — Hackers Fight Club · instagram.com/hackersfightclub · blog.hackersfight.club

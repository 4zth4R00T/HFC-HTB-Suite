# Estado

## Versión 1.1 (2026-11-03)

Se incorporó el núcleo de máquinas de la edición Pro, verificado contra la API
en vivo en las tres implementaciones:

| Función | Rust | Bash | PowerShell |
|---|---|---|---|
| Orden de listas (valoración, fecha, dificultad votada, owns, nombre) y sentido | sí | sí | sí |
| Filtro de completadas y estado sin publicar | sí | sí | sí |
| Favoritos de máquinas, retos y sherlocks | sí | sí | sí |
| Ficha completa: sinopsis, etiquetas, matriz, bloods | sí | sí | sí |
| Opiniones, actividad, registro de cambios | sí | sí | sí |
| Writeup oficial, vídeo y guías de la comunidad | sí | sí | sí |
| IP en vivo dentro de la ficha al lanzar | sí | sí | sí |
| Búsqueda global y perfil público de usuarios | sí | sí | sí |
| Dashboard: en progreso, recomendadas, favoritas | sí | sí | sí |
| Sherlocks con búsqueda, estado, dificultad, categoría y orden | sí | sí | sí |
| Seasons: resumen con cuenta atrás, leaderboard, recompensas | sí | sí | sí |
| Rankings de usuarios, equipos, universidades y países | sí | sí | sí |
| Pwnbox de Labs | sí | sí | sí |
| Skin 0day-notes y paletas nuevas | sí | no | no |

Rutas de la API usadas: `v5 /machines` con `sort_by`, `sort_type`,
`showCompleted`, `todo=1` y `state=unreleased`; `/machine/profile/{id}`,
`/machine/tags/{id}`, `/machine/graph/matrix/{id}`,
`/review/{machine|challenge|sherlock}/{id}/paginated`, `/{kind}/activity/{id}`,
`/{kind}/changelog/{id}`, `/machine/walkthroughs/{id}`, `/machine/writeup/{id}`,
`/machine/todo/update/{id}`, `/challenge/todo/update/{id}`, `/sherlocks` con
`category[]`, `/sherlocks/categories/list`, `/search/fetch`,
`/v5/user/dashboard/{inprogress|recommended|favorites}`,
`/season/players/leaderboard` (máximo 15 por página), `/season/rewards/{id}`,
`/rankings/{users|teams|universities|countries}`, `/pwnbox/status`,
`/pwnbox/usage` y `/user/profile/basic/{uid}`.


Dónde se quedó todo y qué queda pendiente.

## Endurecimiento (2026-09-14)

El token dejó de guardarse en claro. Ahora vive en
`~/.config/htb-hfc/token.vault`: AES-256-CBC con HMAC-SHA256 en Encrypt-then-MAC,
claves derivadas de una passphrase con Argon2id (Rust) o PBKDF2-HMAC-SHA512 de
600 000 rondas (Bash y PowerShell). Las tres versiones comparten el formato y
hay tests en `rust/htb-cli/` que abren vaults reales generados por las otras dos.
Al arrancar, un token en claro de una versión anterior se cifra y se sobrescribe.

Además se cerraron estas vías, explicadas en el README:

- Fuera el OSC 52 de las tres versiones: copiaba flags e IPs al terminal, que
  sobre SSH o tmux es otra máquina.
- El token ya no pasa por `argv` en ningún sitio: la cabecera de `curl` va por
  `--config -`, y `htb token --set` pide el token por teclado.
- Fuera el `sudo apt-get install` automático de la versión Bash.
- Las tres se niegan a correr como root salvo `HTB_ALLOW_ROOT=1`.
- `PATH` saneado, `umask 077`, directorio de configuración forzado a `700` y
  rechazado si es enlace o de otro usuario, `.ovpn` a `600`, temporales fuera
  de `/tmp`.
- El binario de Rust va sin símbolos, con `panic = "abort"` y mensaje genérico,
  sin `--self-test` fuera de desarrollo, sin core dumps y marcado como no
  depurable.

Queda pendiente: no se probó contra la cuenta real después del cambio, solo el
vault y el arranque. La primera sesión real conviene hacerla con un token que
puedas revocar.

## Lo que funciona y está verificado en vivo

Las tres versiones (Rust, Bash, PowerShell) están a la par. Cada cosa de esta
lista se probó contra la cuenta real, no solo compilando.

### Ratón, portapapeles y teclas
- Rueda, clic (el primero marca, el segundo abre) y clic derecho para volver.
- `c` copia lo que ofrezca la pantalla: IP de la máquina, `ip:puerto` más
  `http://ip:puerto` en retos con contenedor, IP de fortress, IP y orden
  `xfreerdp` en sherlocks con VM.
- El prompt de flag se prellena desde el portapapeles si lo que hay tiene forma
  de flag.
- En Rust `Ctrl+C` copia y `Ctrl+Q` cierra; en Bash y PowerShell `Ctrl+C` cierra.
- `g` y `G` ya no son atajos de inicio/fin: se liberaron y ahora Inicio y Fin son
  las teclas de verdad.

### Filtros
- Máquinas: `d` dificultad, `o` sistema, `v` estado (resueltas / sin resolver),
  `x` limpia. Todos son listas de selección, nunca texto libre.
- Challenges: `c` categoría, `d` dificultad, `u` solo sin resolver, `x` limpia.
- La lista de challenges carga el catálogo entero (~853, en páginas de 100). El
  `/challenge/list` de antes solo devolvía 198 y por eso «Web» daba 16.
- Máquinas mandan `difficulty[]` y `os[]` (con corchetes). Sin ellos la API
  responde `The difficulty field must be an array`.

### Starting Point
- Los tres niveles con barra de progreso, y dentro las máquinas de cada uno.
- Ojo: las máquinas de Starting Point están en **otra red VPN**. Si responden al
  ping pero todos los puertos salen filtrados, hay que cambiar el producto a
  `starting_point` en la pantalla de VPN.

### Pasos guiados
- Las preguntas **no** están tras ninguna tecla: son la pantalla de la máquina,
  tanto en su detalle como en «Mi objetivo».
- Cada fila lleva la pregunta y el formato esperado (los asteriscos dicen cuántos
  caracteres). `↵` responde. Las ya contestadas enseñan la respuesta dada.
- Las listas llevan barra con el porcentaje (`taskCompletionPercentage`).
- Las máquinas sin modo guiado enseñan sus casillas de flag user y root.

### Dificultad
- Al enviar flag sale la escala con nombres (Pan comido … Brainfuck), no un
  número a ciegas.
- El detalle muestra la media votada y, con `v`, el desglose en barras.

## Rutas de la API que costó encontrar

Prefijo `/machines` en **plural** y sobre **v4** (el resto del perfil usa
`/machine` en singular, y por eso no aparecían).

| Método | Ruta | Para qué |
|---|---|---|
| GET | `/api/v4/machines/{id}/tasks` | Preguntas del modo guiado |
| POST | `/api/v4/machines/{id}/tasks/{taskId}/flag` | Responder una pregunta, cuerpo `{"flag":"..."}` |
| GET | `/api/v4/machines/{id}/adventure` | Modo clásico, solo user y root |
| GET | `/api/v4/sp/tiers/progress` | Progreso de Starting Point (`/sp/tiers` a secas da 404) |
| GET | `/api/v5/machines?spTier={1,2,3}` | Máquinas de cada nivel |
| GET | `/api/v4/machine/walkthroughs/{id}` | Writeups oficial y de la comunidad |

`/machines/{id}/tasks` da 403 si `isGuidedEnabled` es falso; ahí se cae a
`/adventure`. Si una tarea está completada, la respuesta trae el campo `flag`
con la respuesta en claro.

Contraseñas de los zip: challenges `hackthebox`, **sherlocks `hacktheblue`**.

## Resuelto esta sesión

| Qué | Resultado |
|---|---|
| BabyEncryption (Crypto, Very Easy) | Flag aceptada. Cifrado afín `(123·m+18) mod 256` |
| Unit42 (Sherlock DFIR, Very Easy) | 8/8 respuestas correctas |
| Appointment (SP Tier 2) | Flag `e3d0796d002a446c0e622226f42e9672`, SQLi `admin' or '1'='1'-- -` |
| Sequel (SP Tier 1) | Flag `7b4bec00d1a39e3dd4e021ec3d915da8`, MariaDB con root sin contraseña en 3306 |
| Reactor | **Sin resolver.** Ver abajo |

### Sequel, respuestas del modo guiado
`3306` · `MariaDB` · `-u` · `root` · `*` · `;` · `htb` · `use` · `describe` ·
`config` · flag. Las once constan ya como completadas en la cuenta y coinciden.

### Reactor: dónde se atascó
Solo 22 (OpenSSH 9.6p1) y 3000 (Next.js 15.0.3, App Router). Una única página
estática, sin formularios ni JS de aplicación. Descartado: ~30.000 rutas con
varios diccionarios, `/api/*`, rutas de dos niveles (56.000 combinaciones),
ficheros con extensión, parámetros GET, vhosts, IDs numéricos, manifests
internos, los headers de enrutado (`x-matched-path`, `x-invoke-path`,
`x-now-route-matches`), CVE-2025-29927 con sus cuatro variantes y refuzzeo
completo con el header, WebSocket, traversal en `/_next/image`, IPv6 y UDP.

Lo único anómalo: un POST con `$ACTION_ID_` inválido devuelve **500**, o sea que
hay Server Actions procesándose en `/`. Sin el hash de 40 hex no hay forma de
invocarlas desde fuera. Ese es el camino que queda.

## Pendiente

1. **Responder** (SP Tier 1, Windows, 12 preguntas) quedó lanzada y sin tocar.
   El plan era resolverla con un agente que tenga Bash y traer las respuestas
   antes de enviar nada.
2. Reactor, si se quiere insistir por las Server Actions.
3. La pantalla de votos (`v`) está en las tres, pero solo se probó en Rust.

## Notas de trabajo

- Al delegar en agentes: el `ctf-solver` **no tiene herramienta Bash**, solo
  lectura de ficheros y web. Para atacar máquinas hay que usar `general-purpose`.
- La API está tras Cloudflare con rate limiting agresivo. Espaciar 1,5–2 s entre
  peticiones; si salta «Too Many Attempts», esperar un par de minutos.
- `Htb.I18n.ps1` se genera desde `lib/htb-i18n.sh` con un script de Python cada
  vez que cambian las claves. No editarlo a mano.
- Cuidado en PowerShell: `@($null)` cuenta como un elemento, y `return ,$array`
  mete el array dentro de otro array. Los dos dieron guerra.
- Cuidado en Bash: `read` con IFS de tabulador colapsa los campos vacíos
  consecutivos. Por eso las filas de tareas usan `\x1f` como separador.

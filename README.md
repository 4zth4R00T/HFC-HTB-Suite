# HFC HTB Suite

Un cliente de terminal para Hack The Box, escrito por Aztharot y hecho en casa,
en HFC, Hackers Fight Club.

Más abajo está el mismo texto en inglés. The English version follows below.

## Español

### Qué es esto

HFC HTB Suite es un cliente de consola para Hack The Box Labs. Hace desde la
terminal lo mismo que harías en la web: buscar máquinas activas y retiradas,
ver su ficha, lanzarlas, pararlas, reiniciarlas, extenderles el tiempo y
enviar la flag. También lleva los retos, con su zip y su contenedor que se
levanta y se apaga, las fortalezas, los sherlocks con sus preguntas una a una,
los pro labs, los tracks, las seasons, el Starting Point, la VPN con sus
servidores y su archivo de conexión, tu perfil, tus estadísticas y un
diagnóstico que te dice si la API responde y con cuánta latencia.

Hay tres versiones y las tres hacen lo mismo. La de Rust tiene una interfaz
completa, con ratón, scroll, temas claro y oscuro, y además funciona como
comando suelto para scripts. La de Bash y la de PowerShell existen para cuando
no quieres compilar nada o estás en una máquina que no es tuya. Las tres hablan
con la misma API, comparten la misma carpeta de configuración y entienden el
mismo archivo de token, así que puedes empezar en una y seguir en otra.

Está en español, inglés y ruso. Se cambia desde ajustes y se queda guardado.

### Qué problema resuelve

El primero es tonto y es el que más molesta: el navegador te saca del ritmo.
Estás en la terminal con un escaneo a medias y para lanzar una máquina, copiar
su dirección, mirar cuánto tiempo le queda, bajar el zip de un reto o mandar
una flag tienes que ir a una pestaña, buscar, hacer clic y volver. Parecen diez
segundos y no lo son, porque lo que se rompe es la concentración. La suite deja
todo eso a una tecla de donde ya estabas.

El segundo es el token. El App Token de HTB es una credencial con acceso
completo a tu cuenta, y lo normal, cuando uno se hace un script para esto, es
acabar guardándolo en un archivo de texto en el disco. Aquí vive dentro de un
contenedor cifrado que se abre con una passphrase que solo tienes tú. El
contenido va con AES de 256 bits y por encima lleva una firma que se comprueba
antes de descifrar nada, así que un archivo manipulado se rechaza sin llegar al
descifrador. La clave sale de tu passphrase con Argon2id en la versión de Rust
y con PBKDF2 de 600.000 rondas en las otras dos, y el texto se rellena para que
el tamaño del archivo no cuente cuánto mide el token. Las tres versiones leen
el mismo formato.

El tercero es todo lo que se escapa sin que nadie lo note. Una herramienta que
maneja un token, flags y direcciones de máquinas, y que se ejecuta en la misma
sesión donde estás trabajando, tiene más agujeros de los que parece. Copiar al
portapapeles con la secuencia que entiende el terminal manda el texto a la
máquina que dibuja ese terminal, que sobre SSH o dentro de tmux no tiene por
qué ser la tuya; eso se quitó. Pasarle el token a curl como argumento lo deja
escrito en un sitio del sistema que puede leer cualquier usuario de la máquina;
ahora entra por la entrada estándar. La versión de Bash instalaba sus propias
dependencias llamando al gestor de paquetes con privilegios; ahora solo te dice
qué falta y se calla. Las tres se niegan a arrancar como root. El binario de
Rust va sin símbolos ni información de depuración, no deja volcados de memoria,
se marca como no rastreable y guarda el token en memoria de forma que se borra
sola. Y si encuentra un token en claro de una versión vieja, lo cifra y lo
sobrescribe antes de borrarlo.

El cuarto son las rarezas de la API de HTB, que son muchas y no están escritas
en ningún sitio. Que la dificultad al votar va del uno al diez en máquinas pero
en retos tiene que ser múltiplo de diez. Que los sherlocks no tienen una flag
sino una respuesta por tarea, cada una con su formato. Que los que traen
máquina virtual se levantan por la ruta de los retos, aunque no sean retos. Que
el Starting Point vive en otra red y si te confundes de VPN la máquina responde
al ping pero tiene todos los puertos filtrados. Que la API solo te entrega el
archivo de conexión del servidor que tengas asignado, así que para bajar otro
hay que cambiarse antes. Que Cloudflare te corta si preguntas demasiado
seguido. Cada una de esas cosas costó una tarde y está resuelta por dentro para
que a ti no te cueste ninguna.

### Por qué lo hice

Porque quería estar en la terminal y no en el navegador, y porque la única
manera que conozco de aprender de verdad cómo funciona algo es construirlo.
Cada rareza de la lista de arriba la descubrí chocándome con ella, y arreglarla
me enseñó más que cualquier writeup. Hacer la misma herramienta tres veces, en
Rust, en Bash y en PowerShell, y que las tres se entendieran hasta el punto de
compartir el archivo cifrado del token, fue el ejercicio más útil que he hecho
en mucho tiempo.

Y luego está la razón de verdad, que no es técnica.

### Gracias

Gracias a Lorne, a Ookami y a Pablocro.

No es una fórmula de cortesía. Probablemente yo no estaría en el hacking si
ellos no me hubieran presentado HFC. Lo que empezó como curiosidad se volvió un
sitio donde había gente haciendo cosas y explicándolas, y eso cambia a
cualquiera. Este programa existe porque ellos me metieron en esto, así que en
parte también es suyo.

HFC, Hackers Fight Club.

### Cómo se usa

La versión de Rust se compila y se instala así:

```
cd rust/htb-cli
cargo build --release
sudo install -m755 target/release/htb /usr/local/bin/htb
```

Las otras dos se ejecutan directamente:

```
./htb.sh
pwsh -File ./htb.ps1
```

La de Bash necesita bash 4 o superior con curl, jq, openssl, python3, xxd y
base64. La de PowerShell necesita PowerShell 7 o superior y funciona igual en
Linux, en macOS y en Windows. Si falta algo te lo dice y para.

El token se genera en la web de HTB, en tu perfil, dentro de ajustes, en la
opción de crear App Token. La primera vez el programa te lo pide, te pide
también una passphrase y guarda el token cifrado con ella. A partir de ahí la
passphrase se pide en cada arranque. Si la pierdes no hay manera de
recuperarla: se borra el contenedor, se genera un token nuevo y se empieza otra
vez.

Para moverse, las flechas mueven, enter abre, escape vuelve, la rueda del ratón
hace scroll y la tecla de interrogación enseña la ayuda. Las letras de la barra
de abajo actúan sobre la fila que tengas marcada y siempre llevan la palabra al
lado para no tener que adivinar.

### Una cosa importante

Esta herramienta habla con tu cuenta usando tu token y no hace nada que no
puedas hacer tú desde la web. El token se queda en tu disco, cifrado, y no sale
de tu máquina más que hacia los servidores de HTB.

### Úsalo, rómpelo y mejóralo

En serio. Cógelo, ábrelo, cámbialo, rómpelo a ver por dónde revienta y arregla
lo que encuentres. Si algo está mal escrito o se puede hacer mejor, hazlo
mejor. Si quieres llevártelo a otro lenguaje, llévatelo. Para eso está
publicado.

Lo único que pido es que no me quiten el crédito. Y no es por ver mi nombre en
algo, que eso me da bastante igual. Es porque el crédito es lo que hace que
alguien sepa a quién escribir, y lo que de verdad quiero es poder colaborar con
más gente en más cosas. Si esto te sirve y me lo dices, ya ganamos los dos.

Aztharot, HFC, Hackers Fight Club.

## English

### What this is

HFC HTB Suite is a terminal client for Hack The Box Labs. It does from the
console the same things you would do on the website: search active and retired
machines, read their details, spawn them, stop them, reset them, extend their
time and submit the flag. It also handles challenges, with their zip file and
their container that you start and stop, fortresses, sherlocks with their
questions answered one by one, pro labs, tracks, seasons, Starting Point, the
VPN with its servers and its connection file, your profile, your statistics and
a diagnostics screen that tells you whether the API is answering and how fast.

There are three versions and all three do the same thing. The Rust one has a
full interface, with mouse support, scrolling, light and dark themes, and it
also works as a plain command for scripts. The Bash and PowerShell ones exist
for when you do not want to compile anything or you are on a machine that is
not yours. All three talk to the same API, share the same configuration folder
and understand the same token file, so you can start in one and continue in
another.

It speaks Spanish, English and Russian. You change it in settings and it stays.

### What it solves

The first one is silly and it is the one that annoys the most: the browser
breaks your rhythm. You are in the terminal with a scan halfway through, and to
spawn a machine, copy its address, check how much time it has left, download a
challenge zip or submit a flag you have to go to a tab, search, click and come
back. It looks like ten seconds and it is not, because what breaks is your
concentration. This suite puts all of that one key away from where you already
were.

The second one is the token. The HTB App Token is a credential with full access
to your account, and what usually happens when someone writes a script for this
is that the token ends up sitting in a plain text file on disk. Here it lives
inside an encrypted container that opens with a passphrase only you know. The
contents use 256 bit AES and carry a signature that is verified before anything
is decrypted, so a tampered file is rejected without ever reaching the
decryptor. The key comes from your passphrase through Argon2id in the Rust
version and through PBKDF2 with 600,000 rounds in the other two, and the
plaintext is padded so the file size does not reveal how long the token is. All
three versions read the same format.

The third one is everything that leaks without anyone noticing. A tool that
handles a token, flags and machine addresses, and that runs in the same session
where you are working, has more holes than it looks. Copying to the clipboard
through the terminal escape sequence sends the text to whichever machine draws
that terminal, which over SSH or inside tmux is not necessarily yours; that was
removed. Passing the token to curl as an argument leaves it written in a place
any user of the machine can read; now it goes in through standard input. The
Bash version used to install its own dependencies by calling the package
manager with privileges; now it just tells you what is missing and stays quiet.
All three refuse to run as root. The Rust binary ships without symbols or debug
information, leaves no core dumps, marks itself as not traceable and keeps the
token in memory in a way that wipes itself. And if it finds a plaintext token
from an older version, it encrypts it and overwrites the original before
deleting it.

The fourth one is the quirks of the HTB API, which are many and are not written
down anywhere. That the difficulty rating goes from one to ten on machines but
has to be a multiple of ten on challenges. That sherlocks do not have a single
flag but one answer per task, each with its own format. That the ones with a
virtual machine are started through the challenges route, even though they are
not challenges. That Starting Point lives on a different network, and if you
pick the wrong VPN the machine answers ping but every port comes back filtered.
That the API only hands you the connection file of the server you are currently
assigned to, so to download a different one you have to switch first. That
Cloudflare cuts you off if you ask too often. Each of those cost an afternoon
and is handled inside, so it costs you nothing.

### Why I built it

Because I wanted to be in the terminal and not in the browser, and because the
only way I know to really learn how something works is to build it. Every quirk
in the list above I found by running into it, and fixing it taught me more than
any writeup. Building the same tool three times, in Rust, in Bash and in
PowerShell, and getting all three to understand each other well enough to share
the same encrypted token file, was the most useful exercise I have done in a
long time.

And then there is the real reason, which is not technical.

### Thanks

Thank you to Lorne, to Ookami and to Pablocro.

This is not a politeness formula. I probably would not be into hacking at all
if they had not introduced me to HFC. What started as curiosity turned into a
place where people were doing things and explaining them, and that changes
anyone. This program exists because they got me into this, so part of it is
theirs too.

HFC, Hackers Fight Club.

### How to use it

The Rust version is built and installed like this:

```
cd rust/htb-cli
cargo build --release
sudo install -m755 target/release/htb /usr/local/bin/htb
```

The other two run directly:

```
./htb.sh
pwsh -File ./htb.ps1
```

The Bash one needs bash 4 or newer with curl, jq, openssl, python3, xxd and
base64. The PowerShell one needs PowerShell 7 or newer and works the same on
Linux, macOS and Windows. If something is missing it tells you and stops.

You generate the token on the HTB website, in your profile, under settings, in
the create App Token option. The first time the program asks for it, asks for a
passphrase too and stores the token encrypted with it. From then on the
passphrase is asked at every start. If you lose it there is no way to recover
it: wipe the container, generate a new token and start again.

To move around, the arrows move, enter opens, escape goes back, the mouse wheel
scrolls and the question mark key shows the help. The letters on the bottom bar
act on the row you have selected and always carry the word next to them so you
never have to guess.

### One important thing

This tool talks to your own account using your own token and does nothing you
could not do yourself from the website. The token stays on your disk,
encrypted, and never leaves your machine except towards the HTB servers.

### Use it, break it and improve it

Seriously. Take it, open it, change it, break it to see where it gives, and fix
whatever you find. If something is badly written or can be done better, do it
better. If you want to port it to another language, port it. That is why it is
published.

The only thing I ask is that you do not remove the credit. And it is not about
seeing my name on something, I honestly do not care much about that. It is that
credit is what lets someone know who to write to, and what I really want is to
be able to work with more people on more things. If this is useful to you and
you tell me so, we both win.

Aztharot, HFC, Hackers Fight Club.

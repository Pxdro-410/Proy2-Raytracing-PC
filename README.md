# Skyblock Island Raytracer

**Pedro Caso — 241286**<br>
Proyecto 2 de Gráficas

Skyblock Island Raytracer es un diorama interactivo estilo Minecraft construido en Rust mediante raytracing. La escena representa una isla suspendida dividida en tres dimensiones: Overworld, Nether y End. Cada región tiene relieve, arquitectura, materiales, iluminación y elementos visuales propios, conectados por tres puentes arqueados hacia una torre central.

## Ejecución

Se necesita Rust estable y Cargo. Desde la raíz del proyecto:

```bash
cargo run --release
```

Para usar el renderizado paralelo disponible durante el desarrollo:

```bash
cargo run --release --features parallel
```

El programa carga las texturas PPM y PGM desde `assets/ppm`, los sonidos desde `assets/sound` y crea las capturas en `assets/screenshots`.

## Escena

La isla está construida como una carcasa circular de bloques y se divide por tres abismos visibles. El relieve no es plano: cada dimensión genera alturas discretas con un perfil distinto. La torre central se eleva sobre una plaza circular y conecta los tres sectores mediante puentes de piedra con arcos, barandales y luminarias.

### Overworld

- Casa de madera y piedra con ventanas de vidrio e iluminación interior.
- Río hundido de agua transparente con cascada.
- Árboles de cerezo, troncos, ramas y copas de hojas con transparencia recortada.
- Colinas de césped y tierra que rodean las construcciones sin ocultarlas.

### Nether

- Lago de lava amplio, cascadas hacia el vacío y charcos encajados en el suelo.
- Fortaleza de ladrillos del Nether con pasarela elevada.
- Portal de obsidiana con vidrio magenta refractante y emisivo.
- Relieve abrupto de netherrack y glowstone que ilumina el entorno.

### End

- Santuario y fuente central de salida.
- Ocho torres de obsidiana de distintos tamaños y alturas, distribuidas por el terreno y coronadas por cristales y glowstone.
- Pilares originales con variantes abiertas y protegidas por vidrio.
- Asteroide flotante de End stone y mesetas escalonadas.

## Controles

| Control | Acción |
| --- | --- |
| `A`, `D`, `W`, `S` | Rotar la vista orbital alrededor de la isla. |
| Flechas | Girar la vista sin mover la cámara. |
| `+`, `-` | Acercar o alejar la cámara. |
| `F` | Alternar entre cámara orbital y vuelo libre. |
| `W`, `S`, `A`, `D` en vuelo libre | Avanzar, retroceder y desplazarse lateralmente. |
| `Espacio`, `Shift` en vuelo libre | Subir y bajar. |
| `Q`, `E` | Retroceder o avanzar la hora del día. |
| `R` | Restaurar el mediodía. |
| Clic izquierdo | Colocar el bloque seleccionado o eliminarlo con la herramienta del quinto espacio. |
| Clic derecho | Copiar el material del bloque apuntado al sexto espacio. |
| `1`–`6` | Elegir un espacio de la barra de objetos. |
| `P` | Guardar una captura PPM y usarla como fondo del menú. |
| `Esc` | Regresar al menú principal. |
| `Ctrl` + `C` | Salir del programa. |

La interfaz incluye crosshair, barra de objetos, selector de espacio y un menú de inicio con las opciones **Join World**, **How to Play** y **Exit**. La banda sonora se reproduce en bucle y los botones tienen sonido al presionarse y liberarse.

## Cumplimiento de la rúbrica

| Criterio | Implementación en el proyecto |
| --- | --- |
| Complejidad de la escena | Diorama de tres dimensiones sobre una isla suspendida, relieve sectorial, carcasa inferior, torre monumental, tres puentes arqueados, río, cascada, árboles, casa, fortaleza, portal, lago y cascadas de lava, santuario, asteroide y ocho torres del End. |
| Atractivo visual | Texturas por bloque y por cara cuando aplica, iluminación solar y local, materiales emisivos, transparencias, reflejos, refracción, profundidad de rayos y una composición con tres biomas contrastantes. |
| Rotación y zoom | Cámara orbital con `WASD`, giro de vista con flechas y zoom con `+` y `-`. La distancia está limitada de forma segura entre 5 y 110 unidades. También incluye vuelo libre. |
| Cinco materiales con textura y parámetros propios | Se implementan más de cinco. La tabla siguiente documenta seis materiales evaluables con sus texturas y parámetros de albedo, especularidad, reflectividad y transparencia. |
| Refracción | El agua usa índice de refracción 1.33 y el vidrio magenta del portal usa 1.45. Ambos casos tienen sentido dentro de la escena y se procesan mediante rayos refractados. |
| Reflexión | El trazador calcula rayos reflejados de forma recursiva. El oro, el agua, la lava y el portal magenta tienen reflectividad propia. |
| Skybox | Fondo atmosférico continuo sin geometría visible, con cielo diurno y nocturno, amanecer y atardecer, sol cuadrado, luna y estrellas de tonos grises y celestes por toda la esfera. |

## Materiales evaluables

Cada material utiliza una textura propia de `assets/ppm` y define sus parámetros en [`src/materials.rs`](src/materials.rs). El parámetro especular corresponde al exponente de brillo; la fuerza especular se ajusta además cuando el material lo necesita.

| Material | Textura principal | Albedo | Especular | Reflectividad | Transparencia | Propiedad contextual |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Césped | `grass_block_top.ppm` y `grass_block_side.ppm` | 0.90 | 10 | 0.00 | 0.00 | Caras superior, lateral e inferior diferenciadas. |
| Lava | `lava_still.ppm` | 1.00 | 18 | 0.05 | 0.00 | Emisión naranja y luces locales sobre el lago y cascadas. |
| Oro | `gold_block.ppm` y `gold_block_top.ppm` | 0.85 | 40 | 0.15 | 0.00 | Brillo especular intenso para remates y decoración. |
| Vidrio | `glass.ppm` con `glass.pgm` | 0.35 | 64 | 0.00 | 0.86 | Transparencia con máscara alfa para ventanas y jaulas. |
| Vidrio magenta | `magenta_stained_glass.ppm` con `magenta_stained_glass.pgm` | 0.40 | 64 | 0.05 | 0.70 | Refracción con índice 1.45 y emisión magenta en el portal. |
| Agua | `water_still.ppm` | 0.55 | 72 | 0.05 | 0.55 | Refracción con índice 1.33 y UV globales para evitar cortes entre bloques. |

También se usan, entre otros, cobblestone, smooth stone, piedra, tierra, troncos de roble y cerezo, hojas de cerezo, netherrack, ladrillos del Nether, glowstone, End stone, obsidiana y terracota negra. Sus texturas se cargan de forma explícita en [`src/texture.rs`](src/texture.rs).

## Implementación de raytracing

El mundo se guarda como voxeles indexados por coordenadas enteras. La intersección recorre la cuadrícula con DDA, por lo que el rayo visita las celdas atravesadas en lugar de comparar la escena contra todos los cubos. El trazador incluye:

- Iluminación ambiental, difusa y especular.
- Sombras con sesgo para evitar auto-intersección.
- Luces puntuales derivadas de todos los bloques de glowstone y agrupadas para la lava.
- Emisión propia de glowstone, lava y vidrio magenta.
- Transparencia con máscara alfa para hojas y vidrio.
- Reflexión, refracción y Fresnel con profundidad máxima de tres rebotes.
- Renderizado a 960 × 720 y opción paralela para el desarrollo.

Los componentes principales están en [`src/main.rs`](src/main.rs), [`src/world.rs`](src/world.rs), [`src/ray_intersect.rs`](src/ray_intersect.rs), [`src/materials.rs`](src/materials.rs), [`src/skybox.rs`](src/skybox.rs) y [`src/island.rs`](src/island.rs).

## demostración

pendiente de adjuntar

## Verificación

```bash
cargo test --locked
cargo check --locked --features parallel
```

Las pruebas verifican, entre otros aspectos, la refracción del agua, las transparencias de vidrio y hojas, la continuidad del río, el relieve de cada sector, la distribución e iluminación de las torres del End, la persistencia de bloques colocados y la carga de texturas y audio.

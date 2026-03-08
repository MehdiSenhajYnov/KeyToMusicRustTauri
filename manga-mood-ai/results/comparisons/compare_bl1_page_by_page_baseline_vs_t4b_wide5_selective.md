# Comparatif page par page - BL/1

Legend: `🟩` = match strict, `🟦` = match relaxed, `🟥` = faux.

Couleur calculee sur le **mood**. L intensite reste affichee mais ne change pas la couleur.

| Page | Bench | Expected | Baseline | Nouvelle methode |
|---|---|---|---|---|
| 01.jpg | no | `peaceful:1` | 🟥 `horror:3` | 🟥 `epic:3` |
| 02.jpg | no | `peaceful:1` | 🟥 `epic:3` | 🟥 `epic:3` |
| 03.jpg | yes | `epic:1` | 🟩 `epic:3` | 🟩 `epic:3` |
| 04.jpg | no | `peaceful:1` | 🟥 `tension:2` | 🟥 `tension:2` |
| 05.png | no | `peaceful:1` | 🟥 `tension:3` | 🟥 `tension:3` |
| 06.png | yes | `tension:1` | 🟩 `tension:2` | 🟩 `tension:3` |
| 07.png | yes | `tension:1` | 🟩 `tension:3` | 🟩 `tension:3` |
| 08.jpg | yes | `tension:1` | 🟩 `tension:3` | 🟩 `tension:3` |
| 09.jpg | yes | `tension:2` | 🟩 `tension:3` | 🟩 `tension:3` |
| 10.jpg | yes | `tension:2` | 🟦 `epic:3` | 🟩 `tension:2` |
| 11.jpg | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 12.jpg | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 13.png | yes | `epic:2` | 🟩 `epic:3` | 🟦 `tension:3` |
| 14.png | yes | `epic:2` | 🟩 `epic:3` | 🟦 `tension:3` |
| 15.jpg | yes | `tension:3` | 🟦 `epic:3` | 🟩 `tension:3` |
| 16.jpg | yes | `tension:2` | 🟦 `epic:3` | 🟩 `tension:3` |
| 17.png | yes | `tension:2` | 🟦 `epic:3` | 🟩 `tension:3` |
| 18.png | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 19.png | yes | `tension:2` | 🟦 `epic:3` | 🟦 `epic:3` |
| 20.png | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 21.png | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 22.png | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 23.png | yes | `epic:1` | 🟩 `epic:3` | 🟥 `sadness:3` |
| 24.jpg | yes | `sadness:2` | 🟥 `epic:3` | 🟩 `sadness:3` |
| 25.png | yes | `sadness:2` | 🟩 `sadness:3` | 🟩 `sadness:3` |
| 26.jpg | yes | `sadness:2` | 🟩 `sadness:3` | 🟩 `sadness:3` |
| 27.jpg | yes | `sadness:2` | 🟩 `sadness:3` | 🟩 `sadness:3` |
| 28.jpg | yes | `sadness:2` | 🟩 `sadness:3` | 🟩 `sadness:3` |
| 29.jpg | yes | `sadness:2` | 🟩 `sadness:2` | 🟩 `sadness:3` |
| 30.jpg | yes | `sadness:2` | 🟩 `sadness:3` | 🟩 `sadness:3` |
| 31.jpg | yes | `sadness:3` | 🟩 `sadness:3` | 🟩 `sadness:3` |
| 32.jpg | yes | `sadness:2` | 🟩 `sadness:3` | 🟩 `sadness:3` |
| 33.jpg | yes | `sadness:3` | 🟩 `sadness:3` | 🟩 `sadness:3` |
| 34.jpg | yes | `peaceful:1` | 🟥 `epic:3` | 🟦 `sadness:3` |
| 35.png | yes | `mystery:2` | 🟩 `mystery:2` | 🟥 `sadness:3` |
| 36.png | yes | `mystery:1` | 🟥 `comedy:2` | 🟩 `mystery:2` |
| 37.png | yes | `comedy:1` | 🟩 `comedy:2` | 🟩 `comedy:2` |
| 38.png | yes | `comedy:1` | 🟩 `comedy:2` | 🟩 `comedy:2` |
| 39.png | yes | `comedy:1` | 🟩 `comedy:3` | 🟩 `comedy:2` |
| 40.jpg | yes | `tension:2` | 🟦 `epic:3` | 🟩 `tension:2` |
| 41.jpg | yes | `tension:2` | 🟦 `epic:3` | 🟩 `tension:2` |
| 42.jpg | yes | `tension:3` | 🟦 `epic:3` | 🟦 `epic:3` |
| 43.jpg | yes | `mystery:2` | 🟥 `epic:3` | 🟩 `mystery:2` |
| 44.jpg | yes | `mystery:2` | 🟥 `epic:3` | 🟩 `mystery:3` |
| 45.jpg | yes | `mystery:2` | 🟥 `epic:3` | 🟩 `mystery:2` |
| 46.jpg | yes | `mystery:3` | 🟥 `epic:3` | 🟩 `mystery:2` |
| 47.png | yes | `tension:2` | 🟩 `tension:3` | 🟩 `tension:3` |
| 48.png | yes | `tension:2` | 🟩 `tension:3` | 🟩 `tension:3` |
| 49.jpg | yes | `tension:3` | 🟩 `tension:3` | 🟩 `tension:3` |
| 50.jpg | yes | `tension:3` | 🟩 `tension:3` | 🟩 `tension:3` |
| 51.png | yes | `tension:2` | 🟩 `tension:3` | 🟩 `tension:3` |
| 52.png | yes | `tension:2` | 🟩 `tension:3` | 🟩 `tension:3` |
| 53.jpg | yes | `tension:3` | 🟩 `tension:3` | 🟩 `tension:3` |
| 54.png | yes | `tension:2` | 🟩 `tension:3` | 🟩 `tension:3` |
| 55.png | yes | `tension:3` | 🟥 `comedy:2` | 🟩 `tension:3` |
| 56.jpg | yes | `epic:2` | 🟥 `comedy:2` | 🟦 `tension:2` |
| 57.jpg | yes | `epic:2` | 🟩 `epic:3` | 🟦 `tension:2` |
| 58.png | yes | `tension:3` | 🟥 `horror:3` | 🟩 `tension:2` |
| 59.jpg | yes | `tension:3` | 🟩 `tension:3` | 🟩 `tension:3` |
| 60.png | yes | `epic:2` | 🟦 `tension:3` | 🟦 `tension:3` |
| 61.jpg | yes | `epic:3` | 🟦 `tension:3` | 🟦 `tension:3` |
| 62.jpg | yes | `epic:3` | 🟩 `epic:3` | 🟦 `tension:3` |
| 63.png | yes | `epic:3` | 🟩 `epic:3` | 🟦 `tension:3` |
| 64.jpg | yes | `epic:3` | 🟩 `epic:3` | 🟩 `epic:2` |
| 65.jpg | yes | `epic:3` | 🟩 `epic:3` | 🟩 `epic:2` |
| 66.jpg | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 67.png | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 68.png | yes | `epic:3` | 🟩 `epic:3` | 🟩 `epic:3` |
| 69.jpg | yes | `epic:3` | 🟩 `epic:3` | 🟩 `epic:3` |
| 70.jpg | yes | `epic:3` | 🟩 `epic:3` | 🟩 `epic:3` |
| 71.png | yes | `epic:2` | 🟩 `epic:3` | 🟩 `epic:3` |
| 72.jpg | yes | `tension:2` | 🟦 `epic:3` | 🟦 `epic:3` |
| 73.png | yes | `tension:2` | 🟦 `epic:3` | 🟩 `tension:2` |
| 74.jpg | yes | `tension:2` | 🟦 `epic:3` | 🟩 `tension:2` |

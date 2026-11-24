# EA RTTY Contest - spiegazione del punteggio

| Riga | Cosa fa | Regolamento |
|---|---|---|
| `bands=80,40,20,15,10` | le cinque bande | Bandas |
| `points=...=2; my_dxcc:EA..=1; dxcc:EA..=3; any=1` | EA con EA (anche EA6, EA8, EA9) 2 punti, EA con DX 1, DX con EA 3, DX con DX 1 | Puntuacion |
| `mult=dxcc/band` | ogni entita' DXCC una volta per banda | Multiplicadores |
| `exch/band/if:dxcc:EA,EA6,EA8,EA9` | provincia (o `HQ`) dello scambio delle stazioni EA, una volta per banda | Multiplicadores, nota 3 |
| `area:K,VE,JA,VK/band` | area numerica di W, VE, JA e VK (W5, VE3, JA1, VK2) una volta per banda | Multiplicadores, nota 2 |

Punteggio = somma dei punti di tutte le bande x somma dei moltiplicatori di tutte le bande. Nessuno script.

Cosa il motore non copre: il QSO e' valido solo se compare in almeno due log (confronto con i log degli altri, fuori portata); i QSO unici non valgono; `HQ` vale per tutte le stazioni ma solo EA4URE lo manda.

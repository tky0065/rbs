#!/usr/bin/env python3
"""Régénère la capture du formateur `pretty` affichée dans la documentation.

    python3 docs/scripts/capture_logs_pretty.py

L'image n'est jamais retouchée à la main : elle est rendue depuis la sortie réelle
de `cargo run -p rbs-core --example logs_pretty`, ce qui la rend rejouable après
toute évolution du formateur.
"""

import pathlib
import re
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFont

RACINE = pathlib.Path(__file__).resolve().parents[2]
SORTIE = RACINE / "docs" / "static" / "img" / "logs-pretty.png"
POLICE = "/System/Library/Fonts/Menlo.ttc"

TAILLE = 26
INTERLIGNE = 42
MARGE_X, MARGE_Y = 34, 30
FOND = (30, 32, 40)

# Le formateur n'émet que ces attributs SGR ; tout autre code signale une évolution
# du rendu que cette table ne saurait pas peindre.
COULEURS = {
    "0": None,
    "2": (150, 155, 168),
    "31": (240, 113, 120),
    "32": (152, 195, 121),
    "33": (229, 192, 123),
    "34": (97, 175, 239),
    "90": (128, 134, 150),
}
TEXTE = (222, 226, 235)
SGR = re.compile(r"\x1b\[([0-9;]*)m")
NIVEAUX = ("TRACE", "DEBUG", "INFO", "WARN", "ERROR")


def capturer() -> str:
    """Lance l'exemple derrière un pty : hors TTY le formateur retire ses couleurs."""
    resultat = subprocess.run(
        ["script", "-q", "/dev/null", "cargo", "run", "-q", "-p", "rbs-core",
         "--example", "logs_pretty"],
        cwd=RACINE, capture_output=True, text=True, check=True,
        stdin=subprocess.DEVNULL,
    )
    return resultat.stdout


def segmenter(ligne: str) -> list[tuple[str, tuple[int, int, int]]]:
    """Découpe une ligne en fragments (texte, couleur) d'après ses séquences SGR."""
    segments, position, couleur = [], 0, TEXTE

    for marque in SGR.finditer(ligne):
        if marque.start() > position:
            segments.append((ligne[position:marque.start()], couleur))
        for code in marque.group(1).split(";"):
            if code not in COULEURS:
                raise SystemExit(f"Attribut SGR « {code} » inconnu : la table de couleurs "
                                 "de ce script est en retard sur le formateur.")
            couleur = COULEURS[code] or TEXTE
        position = marque.end()

    if position < len(ligne):
        segments.append((ligne[position:], couleur))
    return segments


def main() -> None:
    # `script(1)` de macOS ouvre sa sortie par la représentation visible d'un EOT.
    brut = re.sub(r"\A\^D", "", capturer())
    # Retire les parasites de `script(1)` (^D, retours arrière) sans toucher à ESC,
    # que la segmentation SGR va lire juste après.
    lignes = [re.sub(r"[\x00-\x08\x0b-\x1a\x1c-\x1f\x7f]", "", ligne).rstrip()
              for ligne in brut.replace("\r\n", "\n").split("\n")]
    lignes = [ligne for ligne in lignes if ligne.strip()]

    nus = [SGR.sub("", ligne) for ligne in lignes]
    manquants = [niveau for niveau in NIVEAUX
                 if not any(niveau in ligne for ligne in nus)]
    if manquants:
        raise SystemExit(f"Niveaux absents de la sortie : {', '.join(manquants)}. "
                         "La capture doit montrer les cinq.")

    police = ImageFont.truetype(POLICE, TAILLE)
    largeur_car = police.getlength("M")
    largeur = int(max(len(ligne) for ligne in nus) * largeur_car) + 2 * MARGE_X
    hauteur = len(lignes) * INTERLIGNE + 2 * MARGE_Y

    image = Image.new("RGB", (largeur, hauteur), FOND)
    pinceau = ImageDraw.Draw(image)

    for index, ligne in enumerate(lignes):
        x, y = MARGE_X, MARGE_Y + index * INTERLIGNE
        for texte, couleur in segmenter(ligne):
            pinceau.text((x, y), texte, font=police, fill=couleur)
            x += police.getlength(texte)

    SORTIE.parent.mkdir(parents=True, exist_ok=True)
    image.save(SORTIE)
    print(f"{SORTIE.relative_to(RACINE)} — {largeur}×{hauteur}, {len(lignes)} lignes")


if __name__ == "__main__":
    sys.exit(main())

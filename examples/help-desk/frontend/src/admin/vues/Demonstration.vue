<script setup lang="ts">
import { ArrowDownIcon, ArrowUpIcon, ChevronLeftIcon, ChevronRightIcon } from '@lucide/vue'
import { computed, ref, watch } from 'vue'

import Bande from '@/components/Bande.vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import {
  Table,
  TableBody,
  TableCell,
  TableEmpty,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { useInterface } from '@/stores/interface'

/**
 * Les libellés de cet écran, rendus dans la langue du projet à sa création.
 *
 * Ici et non dans les libellés du shell : un écran engendré s'ajoute sans qu'aucun
 * fichier partagé n'ait à recevoir de ligne, et le registre d'ancres n'en porte donc que
 * deux pour toute l'administration.
 */
const TEXTES = {
  titre: 'Démonstration',
  sous_titre: 'Cet écran est le patron dont sortent les écrans engendrés. Ses lignes vivent dans le fichier : aucune table ne les porte encore.',
  filtre: 'Filtrer',
  lignes: 'lignes',
  aucune: 'Aucune ligne ne correspond',
  chargement: 'Chargement…',
  erreur: 'La source est injoignable.',
  page: 'Page',
  sur: 'sur',
  precedent: 'Précédent',
  suivant: 'Suivant',
  actions: 'Actions',
  creer: 'Nouvelle ligne',
  modifier: 'Modifier',
  supprimer: 'Supprimer',
  detail: 'Détail',
  enregistrer: 'Enregistrer',
  enregistrement: 'Enregistrement…',
  annuler: 'Annuler',
  suppression_titre: 'Supprimer cette ligne',
  suppression_detail: 'La ligne part de la table, et le geste ne se reprend pas.',
  enregistree: 'Ligne enregistrée.',
  supprimee: 'Ligne supprimée.',
  refus_enregistrement: 'Enregistrement refusé.',
  refus_suppression: 'Suppression refusée.',
  introuvable: 'Ligne introuvable.',
  vide: '—',
  oui: 'Oui',
  non: 'Non',
} as const

/** Combien de lignes une page porte. */
const TAILLE = 5

/** La colonne sur laquelle la table est triée, et dans quel sens. */
interface Tri {
  cle: string
  sens: 'asc' | 'desc'
}

/** Ce que l'écran demande à sa source pour remplir une page. */
interface Requete {
  filtre: string
  tri: Tri | null
  page: number
  taille: number
}

/** Une ligne de la table, une propriété par colonne du corps. */
interface Ligne {
  reference: string
  libelle: string
  etat: 'actif' | 'archivé' | 'brouillon'
  maj: string
}

/** Ce que le formulaire saisit : un booléen par case, une chaîne partout ailleurs. */
interface Formulaire {
  reference: string
  libelle: string
  etat: string
  maj: string
}

/** Un formulaire vierge : ce qu'ouvre la création. */
const VIERGE: Formulaire = {
  reference: '',
  libelle: '',
  etat: 'actif',
  maj: '',
}

/**
 * Ce que la source reçoit, tiré de ce que le formulaire porte.
 *
 * Un champ laissé vide vaut l'absence de valeur et non la chaîne vide : c'est la seule
 * lecture qui laisse remettre à zéro une colonne facultative depuis le formulaire. Un
 * instant repart au fuseau que le contrat attend, et que le contrôle natif ne porte pas.
 */
function corps(formulaire: Formulaire) {
  return {
    reference: formulaire.reference,
    libelle: formulaire.libelle,
    etat: formulaire.etat as 'actif' | 'archivé' | 'brouillon',
    maj: formulaire.maj,
  }
}

/** Le formulaire prérempli d'une ligne existante. */
function saisie(ligne: Ligne): Formulaire {
  return {
    reference: ligne.reference,
    libelle: ligne.libelle,
    etat: ligne.etat,
    maj: ligne.maj,
  }
}

// D'ici à la fin de `raison`, la source des lignes. C'est la seule part de cet écran qui
// dépende d'un contrat : tout ce qui suit — filtre, tri, pagination, formulaire, détail,
// rendu — ne connaît que `Ligne`, `Requete` et `Formulaire`, et ne change pas d'une table
// à l'autre.

const SOURCE: Ligne[] = [
  { reference: 'DEM-001', libelle: 'Bordereau', etat: 'actif', maj: '2026-01-01' },
  { reference: 'DEM-002', libelle: 'Relevé', etat: 'archivé', maj: '2026-02-04' },
  { reference: 'DEM-003', libelle: 'Inventaire', etat: 'brouillon', maj: '2026-03-07' },
  { reference: 'DEM-004', libelle: 'Journal', etat: 'actif', maj: '2026-04-10' },
  { reference: 'DEM-005', libelle: 'Facture', etat: 'archivé', maj: '2026-05-13' },
  { reference: 'DEM-006', libelle: 'Carnet', etat: 'brouillon', maj: '2026-06-16' },
  { reference: 'DEM-007', libelle: 'Registre', etat: 'actif', maj: '2026-01-19' },
  { reference: 'DEM-008', libelle: 'Feuille', etat: 'archivé', maj: '2026-02-22' },
  { reference: 'DEM-009', libelle: 'Ruban', etat: 'brouillon', maj: '2026-03-25' },
  { reference: 'DEM-010', libelle: 'Listing', etat: 'actif', maj: '2026-04-01' },
  { reference: 'DEM-011', libelle: 'Bandeau', etat: 'archivé', maj: '2026-05-04' },
]

async function interroger(requete: Requete): Promise<{ lignes: Ligne[]; total: number }> {
  const motif = requete.filtre.trim().toLowerCase()
  const retenues = SOURCE.filter(
    (ligne) =>
      motif === '' ||
      Object.values(ligne).some((valeur) => String(valeur).toLowerCase().includes(motif)),
  )

  const ordre = requete.tri
  if (ordre !== null) {
    const cle = ordre.cle as keyof Ligne
    const sens = ordre.sens === 'asc' ? 1 : -1
    retenues.sort((gauche, droite) => sens * String(gauche[cle]).localeCompare(String(droite[cle])))
  }

  const debut = (requete.page - 1) * requete.taille

  return { lignes: retenues.slice(debut, debut + requete.taille), total: retenues.length }
}

async function lire(cle: string): Promise<Ligne> {
  const trouvee = SOURCE.find((ligne) => ligne.reference === cle)

  if (trouvee === undefined) {
    throw new Error(TEXTES.introuvable)
  }

  return trouvee
}

async function enregistrer(cle: string | null, formulaire: Formulaire): Promise<void> {
  const rang =
    cle === null ? -1 : SOURCE.findIndex((ligne) => ligne.reference === cle)

  if (rang === -1) {
    SOURCE.push(corps(formulaire))
  } else {
    SOURCE.splice(rang, 1, corps(formulaire))
  }
}

async function supprimer(cle: string): Promise<void> {
  const rang = SOURCE.findIndex((ligne) => ligne.reference === cle)

  if (rang !== -1) {
    SOURCE.splice(rang, 1)
  }
}

/** La phrase que porte une panne, ou `defaut` quand elle n'en porte pas. */
function raison(cause: unknown, defaut: string): string {
  return cause instanceof Error ? cause.message : defaut
}

/**
 * Comment une valeur se lit : telle quelle, en jour, ou en date et heure.
 *
 * Le contrat porte ses horodatages en ISO 8601, que personne ne lit de l'œil. Le type de
 * la propriété ne suffirait pas à les reconnaître — une date y est une chaîne comme une
 * autre — et la table les rendrait bruts.
 */
type Rendu = 'texte' | 'date' | 'instant'

/** Une colonne de la table : ce qu'elle lit d'une ligne, et si l'on peut trier dessus. */
interface Colonne {
  cle: keyof Ligne
  libelle: string
  rendu: Rendu
  triable: boolean
}

const COLONNES: readonly Colonne[] = [
  { cle: 'reference', libelle: 'Référence', rendu: 'texte', triable: true },
  { cle: 'libelle', libelle: 'Libellé', rendu: 'texte', triable: true },
  { cle: 'etat', libelle: 'État', rendu: 'texte', triable: false },
  { cle: 'maj', libelle: 'Mise à jour', rendu: 'date', triable: true },
]

/** Les propriétés du détail : la ligne entière, et non le seul sous-ensemble affiché. */
const PROPRIETES: readonly { cle: keyof Ligne; libelle: string; rendu: Rendu }[] = [
  { cle: 'reference', libelle: 'Référence', rendu: 'texte' },
  { cle: 'libelle', libelle: 'Libellé', rendu: 'texte' },
  { cle: 'etat', libelle: 'État', rendu: 'texte' },
  { cle: 'maj', libelle: 'Mise à jour', rendu: 'date' },
]

const interfaces = useInterface()

const filtre = ref('')
const tri = ref<Tri | null>(null)
const page = ref(1)

const lignes = ref<Ligne[]>([])
const total = ref(0)
const enCours = ref(false)
const faute = ref<string | null>(null)

// Le formulaire porte la clé de la ligne qu'il modifie, ou `null` quand il en crée une :
// c'est la seule chose qui sépare les deux gestes, et un second dialogue les aurait
// dédoublés.
const formulaireOuvert = ref(false)
const modifiee = ref<string | null>(null)
const valeurs = ref<Formulaire>({ ...VIERGE })
const enregistrementEnCours = ref(false)
const fauteFormulaire = ref<string | null>(null)

// Le détail relit la ligne plutôt que de reprendre celle de la table : celle-ci date de
// la dernière page chargée, et une valeur changée depuis se lirait encore à son ancienne.
const detailOuvert = ref(false)
const detaillee = ref<Ligne | null>(null)
const detailEnCours = ref(false)
const fauteDetail = ref<string | null>(null)

const suppressionOuverte = ref(false)
const aSupprimer = ref<string | null>(null)
const suppressionEnCours = ref(false)

const pages = computed(() => Math.max(1, Math.ceil(total.value / TAILLE)))

// Une seule veille, sur la demande entière : le filtre et le tri ramènent eux-mêmes à la
// première page, et veiller sur chacun d'eux séparément relancerait deux fois la même
// interrogation pour un seul geste de l'opérateur.
const requete = computed<Requete>(() => ({
  filtre: filtre.value,
  tri: tri.value,
  page: page.value,
  taille: TAILLE,
}))

watch(requete, () => void charger(), { immediate: true })

async function charger(): Promise<void> {
  enCours.value = true
  faute.value = null

  try {
    const reponse = await interroger(requete.value)
    lignes.value = reponse.lignes
    total.value = reponse.total
  } catch (cause) {
    lignes.value = []
    total.value = 0
    faute.value = raison(cause, TEXTES.erreur)
  } finally {
    enCours.value = false
  }
}

function filtrer(valeur: string | number): void {
  filtre.value = String(valeur)
  page.value = 1
}

// Trois états et non deux : la troisième pression rend la colonne au tri naturel de la
// source, que rien d'autre ne saurait retrouver.
function trier(cle: keyof Ligne): void {
  const courant = tri.value

  if (courant === null || courant.cle !== cle) {
    tri.value = { cle, sens: 'asc' }
  } else if (courant.sens === 'asc') {
    tri.value = { cle, sens: 'desc' }
  } else {
    tri.value = null
  }

  page.value = 1
}

function creer(): void {
  modifiee.value = null
  valeurs.value = { ...VIERGE }
  fauteFormulaire.value = null
  formulaireOuvert.value = true
}

function modifier(ligne: Ligne): void {
  modifiee.value = ligne.reference
  valeurs.value = saisie(ligne)
  fauteFormulaire.value = null
  formulaireOuvert.value = true
}

async function soumettre(): Promise<void> {
  enregistrementEnCours.value = true
  fauteFormulaire.value = null

  try {
    await enregistrer(modifiee.value, valeurs.value)
    formulaireOuvert.value = false
    interfaces.informer(TEXTES.enregistree)
    await charger()
  } catch (cause) {
    fauteFormulaire.value = raison(cause, TEXTES.refus_enregistrement)
  } finally {
    enregistrementEnCours.value = false
  }
}

async function detailler(cle: string): Promise<void> {
  detaillee.value = null
  fauteDetail.value = null
  detailEnCours.value = true
  detailOuvert.value = true

  try {
    detaillee.value = await lire(cle)
  } catch (cause) {
    fauteDetail.value = raison(cause, TEXTES.introuvable)
  } finally {
    detailEnCours.value = false
  }
}

function demander(cle: string): void {
  aSupprimer.value = cle
  suppressionOuverte.value = true
}

async function confirmer(): Promise<void> {
  const cle = aSupprimer.value

  if (cle === null) {
    return
  }

  suppressionEnCours.value = true

  try {
    await supprimer(cle)
    suppressionOuverte.value = false
    interfaces.informer(TEXTES.supprimee)
    await charger()
  } catch (cause) {
    faute.value = raison(cause, TEXTES.refus_suppression)
  } finally {
    suppressionEnCours.value = false
  }
}

/**
 * Le format des jours, à Greenwich et non au fuseau de l'opérateur : une date nue vaut
 * minuit UTC, que tout fuseau à l'ouest ramènerait à la veille.
 */
const JOUR = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'short', timeZone: 'UTC' })

/** L'horodatage rendu lisible, ou tel quel si rien ne sait le lire. */
function horodate(brut: string, format: Intl.DateTimeFormat): string {
  const instant = new Date(brut)

  return Number.isNaN(instant.getTime()) ? brut : format.format(instant)
}

/** Ce que l'opérateur lit d'une valeur : un booléen se dit, une absence se marque. */
function afficher(valeur: string | number | boolean | null, rendu: Rendu): string {
  if (valeur === null) {
    return TEXTES.vide
  }
  if (typeof valeur === 'boolean') {
    return valeur ? TEXTES.oui : TEXTES.non
  }
  if (rendu === 'date') {
    return horodate(String(valeur), JOUR)
  }

  return String(valeur)
}
</script>

<!--
  La table : une bande pour ce qu'elle est, une bande pour ce qu'elle porte. Le tri est
  une flèche et la pagination un chevron — les deux seules icônes admises ici, parce
  qu'elles sont l'affordance du geste et non son ornement. Les trois gestes de ligne sont
  des mots : une corbeille à côté du verbe « supprimer » serait l'ornement qu'on refuse.
-->
<template>
  <div class="flex flex-col border-x border-border">
    <Bande>
      <h1 class="mb-3 text-2xl leading-tight">{{ TEXTES.titre }}</h1>
      <p class="max-w-prose text-muted-foreground">{{ TEXTES.sous_titre }}</p>
    </Bande>

    <Bande>
      <div class="mb-5 flex flex-wrap items-center justify-between gap-3">
        <Input
          class="max-w-xs"
          :model-value="filtre"
          :placeholder="TEXTES.filtre"
          :aria-label="TEXTES.filtre"
          @update:model-value="filtrer"
        />
        <p class="text-sm text-muted-foreground" role="status">
          {{ total }} {{ TEXTES.lignes }}
        </p>
        <Button size="sm" @click="creer">{{ TEXTES.creer }}</Button>
      </div>

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead v-for="colonne in COLONNES" :key="colonne.cle">
              <button
                v-if="colonne.triable"
                type="button"
                class="inline-flex items-center gap-1 uppercase tracking-[0.2em]"
                @click="trier(colonne.cle)"
              >
                {{ colonne.libelle }}
                <ArrowUpIcon
                  v-if="tri?.cle === colonne.cle && tri.sens === 'asc'"
                  class="size-3"
                  aria-hidden="true"
                />
                <ArrowDownIcon
                  v-else-if="tri?.cle === colonne.cle"
                  class="size-3"
                  aria-hidden="true"
                />
              </button>
              <span v-else class="uppercase tracking-[0.2em]">{{ colonne.libelle }}</span>
            </TableHead>
            <TableHead class="text-right uppercase tracking-[0.2em]">
              {{ TEXTES.actions }}
            </TableHead>
          </TableRow>
        </TableHeader>

        <TableBody>
          <TableEmpty v-if="lignes.length === 0" :colspan="COLONNES.length + 1">
            {{ enCours ? TEXTES.chargement : (faute ?? TEXTES.aucune) }}
          </TableEmpty>
          <template v-else>
            <TableRow v-for="ligne in lignes" :key="ligne.reference">
              <TableCell v-for="colonne in COLONNES" :key="colonne.cle">
                {{ afficher(ligne[colonne.cle], colonne.rendu) }}
              </TableCell>
              <TableCell class="whitespace-nowrap text-right">
                <Button variant="ghost" size="sm" @click="detailler(ligne.reference)">
                  {{ TEXTES.detail }}
                </Button>
                <Button variant="ghost" size="sm" @click="modifier(ligne)">
                  {{ TEXTES.modifier }}
                </Button>
                <Button variant="ghost" size="sm" @click="demander(ligne.reference)">
                  {{ TEXTES.supprimer }}
                </Button>
              </TableCell>
            </TableRow>
          </template>
        </TableBody>
      </Table>

      <p v-if="faute && lignes.length > 0" class="mt-5 text-destructive" role="alert">
        {{ faute }}
      </p>

      <div class="mt-5 flex items-center justify-between gap-3">
        <Button variant="outline" size="sm" :disabled="page === 1" @click="page -= 1">
          <ChevronLeftIcon aria-hidden="true" />
          {{ TEXTES.precedent }}
        </Button>
        <p class="text-sm text-muted-foreground">
          {{ TEXTES.page }} {{ page }} {{ TEXTES.sur }} {{ pages }}
        </p>
        <Button variant="outline" size="sm" :disabled="page >= pages" @click="page += 1">
          {{ TEXTES.suivant }}
          <ChevronRightIcon aria-hidden="true" />
        </Button>
      </div>
    </Bande>

    <Dialog v-model:open="formulaireOuvert">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ modifiee === null ? TEXTES.creer : TEXTES.modifier }}</DialogTitle>
          <DialogDescription>{{ TEXTES.titre }}</DialogDescription>
        </DialogHeader>

        <form class="flex flex-col gap-5" novalidate @submit.prevent="soumettre">
          <div class="flex flex-col gap-2">
            <Label for="champ-reference">Référence</Label>
            <Input
              id="champ-reference"
              type="text"
              :model-value="valeurs.reference"
              required
              @update:model-value="(valeur) => (valeurs.reference = String(valeur))"
            />
          </div>
          <div class="flex flex-col gap-2">
            <Label for="champ-libelle">Libellé</Label>
            <Input
              id="champ-libelle"
              type="text"
              :model-value="valeurs.libelle"
              required
              @update:model-value="(valeur) => (valeurs.libelle = String(valeur))"
            />
          </div>
          <div class="flex flex-col gap-2">
            <Label for="champ-etat">État</Label>
            <Select
              :model-value="valeurs.etat"
              @update:model-value="(valeur) => (valeurs.etat = String(valeur))"
            >
              <SelectTrigger id="champ-etat">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="actif">actif</SelectItem>
                <SelectItem value="archivé">archivé</SelectItem>
                <SelectItem value="brouillon">brouillon</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="flex flex-col gap-2">
            <Label for="champ-maj">Mise à jour</Label>
            <Input
              id="champ-maj"
              type="date"
              :model-value="valeurs.maj"
              required
              @update:model-value="(valeur) => (valeurs.maj = String(valeur))"
            />
          </div>

          <p v-if="fauteFormulaire" class="text-destructive" role="alert">
            {{ fauteFormulaire }}
          </p>

          <DialogFooter>
            <Button variant="outline" type="button" @click="formulaireOuvert = false">
              {{ TEXTES.annuler }}
            </Button>
            <Button type="submit" :disabled="enregistrementEnCours">
              {{ enregistrementEnCours ? TEXTES.enregistrement : TEXTES.enregistrer }}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>

    <Sheet v-model:open="detailOuvert">
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{{ TEXTES.detail }}</SheetTitle>
          <SheetDescription>{{ TEXTES.titre }}</SheetDescription>
        </SheetHeader>

        <p v-if="detailEnCours" class="px-4 text-muted-foreground" role="status">
          {{ TEXTES.chargement }}
        </p>
        <p v-else-if="fauteDetail" class="px-4 text-destructive" role="alert">
          {{ fauteDetail }}
        </p>
        <dl v-else-if="detaillee !== null" class="flex flex-col gap-4 px-4">
          <div v-for="propriete in PROPRIETES" :key="propriete.cle" class="flex flex-col gap-1">
            <dt class="text-xs uppercase tracking-[0.2em] text-muted-foreground">
              {{ propriete.libelle }}
            </dt>
            <dd class="break-all">{{ afficher(detaillee[propriete.cle], propriete.rendu) }}</dd>
          </div>
        </dl>
      </SheetContent>
    </Sheet>

    <Dialog v-model:open="suppressionOuverte">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ TEXTES.suppression_titre }}</DialogTitle>
          <DialogDescription>{{ TEXTES.suppression_detail }}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" @click="suppressionOuverte = false">
            {{ TEXTES.annuler }}
          </Button>
          <Button variant="destructive" :disabled="suppressionEnCours" @click="confirmer">
            {{ TEXTES.supprimer }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>

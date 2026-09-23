<script setup lang="ts">
import type { CommentaireResponse } from '@/api/client'

import { ArrowDownIcon, ArrowUpIcon, ChevronLeftIcon, ChevronRightIcon } from '@lucide/vue'
import { computed, ref, watch } from 'vue'

import { api, phrase } from '@/api'
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
  titre: 'Commentaires',
  sous_titre: 'Les lignes de la table commentaires, lues et écrites par les routes que ce projet expose.',
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
const TAILLE = 20

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
  id: string
  corps: string
  ticket_id: string
  auteur_id: string
  created_at: string
  updated_at: string
}

/** Ce que le formulaire saisit : un booléen par case, une chaîne partout ailleurs. */
interface Formulaire {
  corps: string
  ticket_id: string
  auteur_id: string
}

/** Un formulaire vierge : ce qu'ouvre la création. */
const VIERGE: Formulaire = {
  corps: '',
  ticket_id: '',
  auteur_id: '',
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
    corps: formulaire.corps,
    ticket_id: formulaire.ticket_id,
    auteur_id: formulaire.auteur_id,
  }
}

/** Le formulaire prérempli d'une ligne existante. */
function saisie(ligne: Ligne): Formulaire {
  return {
    corps: ligne.corps,
    ticket_id: ligne.ticket_id,
    auteur_id: ligne.auteur_id,
  }
}

// D'ici à la fin de `raison`, la source des lignes. C'est la seule part de cet écran qui
// dépende d'un contrat : tout ce qui suit — filtre, tri, pagination, formulaire, détail,
// rendu — ne connaît que `Ligne`, `Requete` et `Formulaire`, et ne change pas d'une table
// à l'autre.

/** La ligne que la table affiche, tirée du corps que le contrat rend. */
function depuis(recu: CommentaireResponse): Ligne {
  return {
    id: recu.id,
    corps: recu.corps,
    ticket_id: recu.ticket_id,
    auteur_id: recu.auteur_id,
    created_at: recu.created_at,
    updated_at: recu.updated_at,
  }
}

async function interroger(requete: Requete): Promise<{ lignes: Ligne[]; total: number }> {
  const ordre = requete.tri
  const motif = requete.filtre.trim()

  // La route de filtre et non celle de liste : elle porte les conditions dans son corps,
  // et garde ses pages même quand la liste pagine par curseur.
  const recue = await api.commentairesFilter(
    {
      sort: ordre === null ? undefined : [ordre.sens === 'asc' ? ordre.cle : `-${ordre.cle}`],
      corps: motif === '' ? undefined : { contains: motif },
    },
    { page: requete.page, per_page: requete.taille },
  )

  return { lignes: recue.data.map(depuis), total: recue.meta.total }
}

async function lire(cle: string): Promise<Ligne> {
  return depuis(await api.commentairesFind(cle))
}

async function enregistrer(cle: string | null, formulaire: Formulaire): Promise<void> {
  if (cle === null) {
    await api.commentairesCreate(corps(formulaire))
  } else {
    await api.commentairesUpdate(cle, corps(formulaire))
  }
}

async function supprimer(cle: string): Promise<void> {
  await api.commentairesDelete(cle)
}

/** La phrase que porte une panne, ou `defaut` quand elle n'en porte pas. */
function raison(cause: unknown, defaut: string): string {
  return phrase(cause, defaut)
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
  { cle: 'corps', libelle: 'Corps', rendu: 'texte', triable: true },
  { cle: 'ticket_id', libelle: 'Ticket id', rendu: 'texte', triable: true },
  { cle: 'auteur_id', libelle: 'Auteur id', rendu: 'texte', triable: true },
  { cle: 'updated_at', libelle: 'Mise à jour', rendu: 'instant', triable: true },
]

/** Les propriétés du détail : la ligne entière, et non le seul sous-ensemble affiché. */
const PROPRIETES: readonly { cle: keyof Ligne; libelle: string; rendu: Rendu }[] = [
  { cle: 'id', libelle: 'Identifiant', rendu: 'texte' },
  { cle: 'corps', libelle: 'Corps', rendu: 'texte' },
  { cle: 'ticket_id', libelle: 'Ticket id', rendu: 'texte' },
  { cle: 'auteur_id', libelle: 'Auteur id', rendu: 'texte' },
  { cle: 'created_at', libelle: 'Créé le', rendu: 'instant' },
  { cle: 'updated_at', libelle: 'Mise à jour', rendu: 'instant' },
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
  modifiee.value = ligne.id
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
 * Le format des instants, construit une fois : un `Intl.DateTimeFormat` neuf par cellule
 * coûterait sa table de locale à chaque rendu de page.
 *
 * Le fuseau est celui de l'opérateur — un horodatage se lit à l'heure où l'on est, pas à
 * Greenwich — mais la locale est celle du projet, comme tous les libellés de cet écran.
 */
const INSTANT = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'short', timeStyle: 'short' })

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
  if (rendu === 'instant') {
    return horodate(String(valeur), INSTANT)
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
            <TableRow v-for="ligne in lignes" :key="ligne.id">
              <TableCell v-for="colonne in COLONNES" :key="colonne.cle">
                {{ afficher(ligne[colonne.cle], colonne.rendu) }}
              </TableCell>
              <TableCell class="whitespace-nowrap text-right">
                <Button variant="ghost" size="sm" @click="detailler(ligne.id)">
                  {{ TEXTES.detail }}
                </Button>
                <Button variant="ghost" size="sm" @click="modifier(ligne)">
                  {{ TEXTES.modifier }}
                </Button>
                <Button variant="ghost" size="sm" @click="demander(ligne.id)">
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
            <Label for="champ-corps">Corps</Label>
            <Input
              id="champ-corps"
              type="text"
              :model-value="valeurs.corps"
              required
              @update:model-value="(valeur) => (valeurs.corps = String(valeur))"
            />
          </div>
          <div class="flex flex-col gap-2">
            <Label for="champ-ticket_id">Ticket id</Label>
            <Input
              id="champ-ticket_id"
              type="text"
              :model-value="valeurs.ticket_id"
              required
              @update:model-value="(valeur) => (valeurs.ticket_id = String(valeur))"
            />
          </div>
          <div class="flex flex-col gap-2">
            <Label for="champ-auteur_id">Auteur id</Label>
            <Input
              id="champ-auteur_id"
              type="text"
              :model-value="valeurs.auteur_id"
              required
              @update:model-value="(valeur) => (valeurs.auteur_id = String(valeur))"
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

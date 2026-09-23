<script setup lang="ts">
import { PlusIcon } from '@lucide/vue'
import { ref } from 'vue'
import { toast } from 'vue-sonner'

import { RouterLink } from 'vue-router'

import Bande from '@/components/Bande.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet'
import { Skeleton } from '@/components/ui/skeleton'
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableEmpty,
  TableFooter,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { TEXTES } from '@/views/galerie-textes'

/** Le nom du projet, tel que `rbs new` l'a fixé. */
const PROJET = 'help-desk'

const texte = ref('port 8080')
const coche = ref(true)
const colonne = ref(true)
const tri = ref('recent')

// Le panneau vient du bord qu'on lui donne, et c'est son affordance principale : le
// montrer d'un seul côté laisserait croire qu'il n'en a qu'un.
const BORDS = [
  { side: 'right', libelle: TEXTES.bord_droit },
  { side: 'left', libelle: TEXTES.bord_gauche },
] as const

const LIGNES = [
  { id: '1', nom: 'health', methode: 'GET', etat: TEXTES.ouverte },
  { id: '2', nom: 'sessions', methode: 'GET', etat: TEXTES.fermee },
  { id: '3', nom: 'sessions', methode: 'DELETE', etat: TEXTES.fermee },
]
</script>

<template>
  <main class="mx-auto max-w-4xl border-x border-border">
    <Bande>
      <p class="mb-6 uppercase tracking-[0.35em] text-muted-foreground">{{ PROJET }}</p>
      <h1 class="mb-4 text-3xl leading-tight">{{ TEXTES.titre }}</h1>
      <p class="mb-3 text-muted-foreground">{{ TEXTES.sous_titre }}</p>
      <p class="text-muted-foreground">{{ TEXTES.theme }}</p>
    </Bande>

    <Bande :titre="TEXTES.bouton">
      <div class="flex flex-wrap items-center gap-3">
        <Button>{{ TEXTES.enregistrer }}</Button>
        <Button variant="secondary">{{ TEXTES.annuler }}</Button>
        <Button variant="outline">{{ TEXTES.exporter }}</Button>
        <Button variant="ghost">{{ TEXTES.ignorer }}</Button>
        <Button variant="link">{{ TEXTES.en_savoir_plus }}</Button>
        <Button variant="destructive">{{ TEXTES.revoquer }}</Button>
        <Button size="sm">{{ TEXTES.petit }}</Button>
        <Button size="lg">{{ TEXTES.grand }}</Button>
        <Button size="icon" :aria-label="TEXTES.ajouter"><PlusIcon /></Button>
        <Button disabled>{{ TEXTES.indisponible }}</Button>
      </div>
    </Bande>

    <Bande :titre="TEXTES.champ">
      <div class="grid gap-3 sm:grid-cols-2">
        <Input v-model="texte" />
        <Input placeholder="127.0.0.1" />
        <Input disabled :placeholder="TEXTES.verrouille" />
        <Input aria-invalid="true" model-value="port -1" />
      </div>
    </Bande>

    <Bande :titre="TEXTES.etiquette">
      <div class="grid max-w-sm gap-2">
        <Label for="hote">{{ TEXTES.hote }}</Label>
        <Input id="hote" placeholder="127.0.0.1" />
      </div>
    </Bande>

    <Bande :titre="TEXTES.carte">
      <Card class="max-w-sm">
        <CardHeader>
          <CardTitle>{{ TEXTES.sonde }}</CardTitle>
          <CardDescription>{{ TEXTES.releve }}</CardDescription>
          <CardAction><Badge variant="outline">{{ TEXTES.ouverte }}</Badge></CardAction>
        </CardHeader>
        <CardContent>
          <p class="text-muted-foreground">{{ TEXTES.base_jointe }}</p>
        </CardContent>
        <CardFooter>
          <Button variant="outline" size="sm">{{ TEXTES.interroger }}</Button>
        </CardFooter>
      </Card>
    </Bande>

    <Bande :titre="TEXTES.badge">
      <div class="flex flex-wrap items-center gap-3">
        <Badge>{{ TEXTES.ouverte }}</Badge>
        <Badge variant="secondary">{{ TEXTES.brouillon }}</Badge>
        <Badge variant="outline">{{ TEXTES.fermee }}</Badge>
        <Badge variant="destructive">{{ TEXTES.revoquee }}</Badge>
      </div>
    </Bande>

    <Bande :titre="TEXTES.separateur">
      <div class="max-w-sm">
        <p class="text-muted-foreground">{{ TEXTES.au_dessus }}</p>
        <Separator class="my-4" />
        <div class="flex h-6 items-center gap-4">
          <span>{{ TEXTES.lecture }}</span>
          <Separator orientation="vertical" />
          <span>{{ TEXTES.ecriture }}</span>
          <Separator orientation="vertical" />
          <span>{{ TEXTES.purge }}</span>
        </div>
      </div>
    </Bande>

    <Bande :titre="TEXTES.table">
      <Table>
        <TableCaption>{{ TEXTES.routes_montees }}</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>{{ TEXTES.route }}</TableHead>
            <TableHead>{{ TEXTES.methode }}</TableHead>
            <TableHead class="text-right">{{ TEXTES.etat }}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow v-for="ligne in LIGNES" :key="ligne.id">
            <TableCell>/{{ ligne.nom }}</TableCell>
            <TableCell>{{ ligne.methode }}</TableCell>
            <TableCell class="text-right">{{ ligne.etat }}</TableCell>
          </TableRow>
        </TableBody>
        <TableFooter>
          <TableRow>
            <TableCell colspan="2">{{ TEXTES.total }}</TableCell>
            <TableCell class="text-right">3</TableCell>
          </TableRow>
        </TableFooter>
      </Table>

      <Table class="mt-8">
        <TableHeader>
          <TableRow>
            <TableHead>{{ TEXTES.session }}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableEmpty>{{ TEXTES.aucune_session }}</TableEmpty>
        </TableBody>
      </Table>
    </Bande>

    <Bande :titre="TEXTES.dialogue">
      <Dialog>
        <DialogTrigger as-child><Button variant="outline">{{ TEXTES.revoquer_session }}</Button></DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{{ TEXTES.revoquer_session }}</DialogTitle>
            <DialogDescription>
              {{ TEXTES.revoquer_detail }}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose as-child><Button variant="ghost">{{ TEXTES.annuler }}</Button></DialogClose>
            <DialogClose as-child><Button variant="destructive">{{ TEXTES.revoquer }}</Button></DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Bande>

    <Bande :titre="TEXTES.menu">
      <DropdownMenu>
        <DropdownMenuTrigger as-child><Button variant="outline">{{ TEXTES.colonnes }}</Button></DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuLabel>{{ TEXTES.affichage }}</DropdownMenuLabel>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem>
              {{ TEXTES.rafraichir }}
              <DropdownMenuShortcut>Ctrl R</DropdownMenuShortcut>
            </DropdownMenuItem>
            <DropdownMenuCheckboxItem v-model="colonne">{{ TEXTES.methode }}</DropdownMenuCheckboxItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>{{ TEXTES.exporter }}</DropdownMenuSubTrigger>
            <DropdownMenuSubContent>
              <DropdownMenuItem>CSV</DropdownMenuItem>
              <DropdownMenuItem>JSON</DropdownMenuItem>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
        </DropdownMenuContent>
      </DropdownMenu>
    </Bande>

    <Bande :titre="TEXTES.panneau">
      <div class="flex flex-wrap gap-3">
        <Sheet v-for="bord in BORDS" :key="bord.side">
          <SheetTrigger as-child>
            <Button variant="outline">{{ TEXTES.ouvrir_detail }}, {{ bord.libelle }}</Button>
          </SheetTrigger>
          <SheetContent :side="bord.side">
            <SheetHeader>
              <SheetTitle>{{ TEXTES.session_une }}</SheetTitle>
              <SheetDescription>{{ TEXTES.session_detail }}</SheetDescription>
            </SheetHeader>
          </SheetContent>
        </Sheet>
      </div>
    </Bande>

    <Bande :titre="TEXTES.liste">
      <div class="flex flex-wrap gap-3">
        <Select v-model="tri">
          <SelectTrigger class="w-64"><SelectValue :placeholder="TEXTES.trier_par" /></SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectLabel>{{ TEXTES.ordre }}</SelectLabel>
              <SelectItem value="recent">{{ TEXTES.plus_recentes }}</SelectItem>
              <SelectItem value="ancien">{{ TEXTES.plus_anciennes }}</SelectItem>
              <SelectItem value="nom">{{ TEXTES.par_nom }}</SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
        <Select disabled>
          <SelectTrigger class="w-64"><SelectValue :placeholder="TEXTES.verrouille" /></SelectTrigger>
          <SelectContent />
        </Select>
      </div>
    </Bande>

    <Bande :titre="TEXTES.case_a_cocher">
      <div class="grid gap-3">
        <div class="flex items-center gap-2">
          <Checkbox id="active" v-model="coche" />
          <Label for="active">{{ TEXTES.session_active }}</Label>
        </div>
        <div class="flex items-center gap-2">
          <Checkbox id="courante" :model-value="false" />
          <Label for="courante">{{ TEXTES.session_courante }}</Label>
        </div>
        <div class="flex items-center gap-2">
          <Checkbox id="verrou" disabled />
          <Label for="verrou">{{ TEXTES.session_verrouillee }}</Label>
        </div>
      </div>
    </Bande>

    <Bande :titre="TEXTES.squelette">
      <div class="grid max-w-sm gap-3">
        <Skeleton class="h-9 w-9" />
        <Skeleton class="h-6 w-40" />
        <Skeleton class="h-4 w-full" />
        <Skeleton class="h-4 w-2/3" />
      </div>
    </Bande>

    <Bande :titre="TEXTES.notifications">
      <div class="flex flex-wrap gap-3">
        <Button variant="outline" @click="toast.success(TEXTES.session_revoquee)">{{ TEXTES.succes }}</Button>
        <Button variant="outline" @click="toast.error(TEXTES.pas_de_reponse)">{{ TEXTES.erreur }}</Button>
        <Button
          variant="outline"
          @click="toast(TEXTES.migration_appliquee, { description: 'm20260919_000001_create_session' })"
        >
          {{ TEXTES.avec_description }}
        </Button>
      </div>
    </Bande>

    <Bande>
      <RouterLink to="/" class="underline underline-offset-4">{{ TEXTES.retour }}</RouterLink>
    </Bande>
  </main>
</template>

# Le client de help-desk

Une application Vue 3, servie par le binaire une fois construite. `npm install`, puis
`npm run dev` le temps du développement, ou `npm run build` pour que le binaire la serve.
Rien ici n'est réengendré : chaque fichier vous appartient, les composants d'interface
compris.

## Où vit l'apparence

Dans un seul bloc. `src/assets/main.css` s'ouvre sur un bloc `@theme` qui porte tous les
jetons que les composants consomment — sept couleurs, une échelle de rayons, une chasse.
Aucun composant ne nomme une couleur. Réécrire ce bloc réécrit l'identité, et `/galerie`
en montre le résultat sur les quatorze composants d'un coup.

Les composants de `src/components/ui` viennent de shadcn-vue et ont été figés ici plutôt
que tirés de lui : le manifeste ne porte aucune dépendance vers la commande qui a servi à
les produire, et le thème que cette commande importe depuis son propre paquet est écrit
en clair ci-dessus. La contrepartie est qu'un correctif amont ne vous parvient pas seul.

## Trois versions tenues une majeure en arrière

Le manifeste épingle trois paquets à l'écart du dernier stable. Chacun tient sur une
incompatibilité vérifiée, et non par prudence — vérifiez que l'incompatibilité a disparu
avant d'en relever un.

- **`typescript`** — la 7 ne publie plus `typescript/lib/tsc`, que `vue-tsc` charge pour
  envelopper le compilateur. `npm run typecheck` s'arrête alors sur
  `ERR_PACKAGE_PATH_NOT_EXPORTED` avant d'avoir lu une ligne. La même note est dans
  `tsconfig.json`.
- **`@vueuse/core`** — `reka-ui`, sur quoi reposent les composants, en dépend en `^14`.
  Demander la 15 à la racine laisse les deux copies dans l'arbre, et donc dans le bundle.
- **`@vue/devtools-api`** — rien ne l'importe. C'est un pair *non optionnel* de Pinia 4 ;
  l'omettre reviendrait à faire dépendre l'installation de la façon dont l'installateur
  résout les pairs.


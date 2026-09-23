import { createPinia } from 'pinia'
import { createApp } from 'vue'

import App from './App.vue'
import { appliquerTheme, themeInitial } from './lib/theme'
import { router } from './router'

import './assets/main.css'

// Avant le montage, et non dans un composant : la classe posée après le premier rendu
// ferait apparaître la page en clair le temps d'une image, sur une machine réglée en
// sombre.
appliquerTheme(themeInitial())

// Pinia est monté ici alors qu'aucun store n'existe encore : c'est la couche d'état du
// socle, et l'y poser une fois évite que chaque écran qui viendra s'y ajouter ait à se
// demander si elle est là.
createApp(App).use(createPinia()).use(router).mount('#app')

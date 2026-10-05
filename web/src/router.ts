import { createRouter, type Router, type RouterHistory } from 'vue-router'
import NotFoundPage from './pages/NotFoundPage.vue'
import OverviewPage from './pages/OverviewPage.vue'

export function createAppRouter(history: RouterHistory): Router {
  return createRouter({
    history,
    routes: [
      { path: '/', name: 'overview', component: OverviewPage },
      { path: '/:pathMatch(.*)*', name: 'not-found', component: NotFoundPage },
    ],
  })
}

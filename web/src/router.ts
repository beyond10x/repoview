import { createRouter, type Router, type RouterHistory } from 'vue-router'
import NotFoundPage from './pages/NotFoundPage.vue'
import OverviewPage from './pages/OverviewPage.vue'
import PlanArtifactPage from './pages/PlanArtifactPage.vue'
import PlanBoardPage from './pages/PlanBoardPage.vue'
import PlanTreePage from './pages/PlanTreePage.vue'
import QualityPage from './pages/QualityPage.vue'
import RepositoryPage from './pages/RepositoryPage.vue'
import SpecRootPage from './pages/SpecRootPage.vue'
import SpecsPage from './pages/SpecsPage.vue'

export function createAppRouter(history: RouterHistory): Router {
  return createRouter({
    history,
    routes: [
      { path: '/', name: 'overview', component: OverviewPage },
      { path: '/plan', name: 'plan-board', component: PlanBoardPage },
      { path: '/plan/tree', name: 'plan-tree', component: PlanTreePage },
      { path: '/plan/artifact/:id', name: 'plan-artifact', component: PlanArtifactPage },
      { path: '/specs', name: 'specs', component: SpecsPage },
      { path: '/specs/:root(.*)', name: 'spec-root', component: SpecRootPage },
      { path: '/quality', name: 'quality', component: QualityPage },
      { path: '/repository', name: 'repository', component: RepositoryPage },
      { path: '/:pathMatch(.*)*', name: 'not-found', component: NotFoundPage },
    ],
  })
}

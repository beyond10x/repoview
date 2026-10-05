import { createApp } from 'vue'
import { createWebHashHistory, createWebHistory } from 'vue-router'
import App from './App.vue'
import { captureToken, readMode } from './api/snapshot'
import { createAppRouter } from './router'
import './style.css'

captureToken()

const history = readMode() === 'static' ? createWebHashHistory() : createWebHistory()

createApp(App).use(createAppRouter(history)).mount('#app')

import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import './index.css'

import HomePage from './components/HomePage.vue'
import ServiceDetail from './components/ServiceDetail.vue'
import LiveLogs from './components/LiveLogs.vue'
import Layout from './components/Layout.vue'

const router = createRouter({
  history: createWebHistory('/dashboard'),
  routes: [
    {
      path: '/',
      component: Layout,
      children: [
        { path: '', component: HomePage },
        { path: 'services/:serviceId', component: ServiceDetail },
        { path: 'logs', component: LiveLogs },
      ],
    },
  ],
})

const app = createApp(App)
app.use(router)
app.mount('#app')

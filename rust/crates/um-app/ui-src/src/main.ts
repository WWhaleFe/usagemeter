import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { start } from './lib/api.svelte';

start();
mount(App, { target: document.getElementById('app')! });

<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";

const taskName = ref("");

async function start() {
  console.log("Starting task", taskName.value)
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  await invoke("start_task", { task: taskName.value });
}


</script>

<template>
  <form class="row" @submit.prevent="start">
    <input id="task-input" v-model="taskName" placeholder="Enter a task..." />
    <button type="submit">Start</button>
  </form>
</template>

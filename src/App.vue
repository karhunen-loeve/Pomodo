<script setup lang="ts">


// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import Task from "./components/Task.vue";
import PomodoTimer from "./components/PomodoTimer.vue";



</script>

<template>
  <div class="container">
    <h1>Pomodo</h1>

    <Suspense>
      <PomodoTimer />
    </Suspense>
    <Task />



  </div>
</template>

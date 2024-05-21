<script setup lang="ts">
import { computed} from 'vue';
import { ref } from "vue";
import { listen } from '@tauri-apps/api/event';


let duration = ref(25 * 60);
let pastTime = ref(0);
let stateText = ref('');
let state = ref('Idle');

function formatTime(time: number) {
    const minutes = Math.floor(time / 60);
    const seconds = Math.floor(time % 60);
    return `${String(minutes).padStart(2,'0')}:${String(seconds).padStart(2, '0')}`;
}

const ratio = computed(() => 1.0 - pastTime.value / duration.value);
const time = computed(() => formatTime(duration.value - pastTime.value));
const circumference = 2 * Math.PI * 36;

await listen('state-changed', (event) => {
    pastTime.value = 0;
    if (typeof event.payload === 'string'){
        if (event.payload === 'Running'){
            stateText.value = '';
            duration.value = 25 * 60;
        } else if (event.payload === 'Paused'){
            stateText.value = 'Pause';
            duration.value = 5 * 60;
        } else if (event.payload === 'Idle'){
            stateText.value = '';
        }
        state.value = event.payload;
    }
});

await listen('tick', (tick) => {
    if (typeof tick.payload === 'number'){
        pastTime.value = tick.payload;
    }
});

</script>

<style scoped>
.timer-progress{
    --time-ratio: v-bind('ratio');
    --circumference: v-bind('circumference');
    fill: none;
    stroke: #8f8;
    stroke-width: 7;
    stroke-linecap: round;
    stroke-dasharray: calc(2 * 3.141592653589793 * 36);
    stroke-dashoffset: calc(2 * 3.141592653589793 * 36 * var(--time-ratio));
    transition: 0.7s;
    opacity: 0.5;
    filter: drop-shadow(0px 0px 2px rgba(177, 247, 186, 0.856));
}

.timer-progress-paused{
    --time-ratio: v-bind('ratio');
    --circumference: v-bind('circumference');
    fill: none;
    stroke: rgb(253, 96, 96);
    stroke-width: 7;
    stroke-linecap: round;
    stroke-dasharray: calc(2 * 3.141592653589793 * 36);
    stroke-dashoffset: calc(2 * 3.141592653589793 * 36 * var(--time-ratio));
    transition: 0.7s;
    opacity: 0.8;
    filter: drop-shadow(0px 0px 2px rgba(247, 177, 177, 0.856));
}

.timer-progress-idle{
    --time-ratio: v-bind('ratio');
    --circumference: v-bind('circumference');
    fill: #f8f8f8;
    transition: 0.7s;
    opacity: 0.5;
}
</style>


<template>
    <div class="timer">
        <svg width="60%" viewBox="0 0 100 100"  xmlns="http://www.w3.org/2000/svg">
            <rect class="timer-rect" width="98" height="98" x="1" y="1" rx="5" ry="5"/>
            <circle class="timer-circle" r="36" cx="50" cy="50" />
            <g transform="rotate(-90 50 50)">
                <circle v-bind:class="{
                    'timer-progress': state == 'Running',
                    'timer-progress-paused': state == 'Paused',
                    'timer-progress-idle': state == 'Idle'
                    }" r="36" cx="50" cy="50"/>
            </g>
        </svg>
        <div class="timer-inner">
            <h1 class="timer-text">{{time}}</h1>
            <h2 class="timer-state">{{ stateText }}</h2>
        </div>
    </div>
</template>

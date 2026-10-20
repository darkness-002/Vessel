<script lang="ts">
  export let size: number = 28;
  export let className: string = '';
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 48 48"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  class={className}
>
  <!-- Sleek rounded squircle background -->
  <rect width="48" height="48" rx="12" fill="#12151E" />
  <rect x="0.5" y="0.5" width="47" height="47" rx="11.5" stroke="#FFFFFF" stroke-opacity="0.1" />

  <!-- Clean, bold minimalist V mark -->
  <path
    d="M13 14L24 35L35 14H29.5L24 25.5L18.5 14H13Z"
    fill="#FFFFFF"
  />
</svg>

<script lang="ts">
    export let label: string;
    export let icon: string = '';
    export let on: any = {};
    export let disabled: boolean = false;
    export let loading: boolean = false;
    export let variant: 'primary' | 'secondary' | 'success' | 'danger' = 'primary';
    export let status: 'ready' | 'disabled' | 'error' | 'none' = 'none';
</script>

<button 
    class="action-button"
    class:disabled
    class:loading
    class:ready={status === 'ready'}
    class:error={status === 'error'}
    {...on}
    disabled={disabled || loading}
>
    {#if loading}
        <span class="spinner"></span>
    {:else}
        {#if icon}
            <span class="button-icon">{icon}</span>
        {/if}
        <span class="button-label">{label}</span>
    {/if}
    
    {#if status === 'ready'}
        <span class="status-indicator ready"></span>
    {/if}
    {#if status === 'error'}
        <span class="status-indicator error"></span>
    {/if}
</button>

<style>
    .action-button {
        position: relative;
        padding: 0.75rem 1.5rem;
        border: none;
        border-radius: 8px;
        font-size: 1rem;
        font-weight: 600;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        transition: all 0.2s ease;
        box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
        overflow: hidden;
    }
    
    .action-button:not(:disabled):hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 10px rgba(0, 0, 0, 0.15);
    }
    
    .action-button:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
    
    .action-button.loading {
        cursor: wait;
    }
    
    /* Variants */
    .action-button {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
    }
    
    .action-button[variant="secondary"] {
        background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
        color: white;
    }
    
    .action-button[variant="success"] {
        background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
        color: white;
    }
    
    .action-button[variant="danger"] {
        background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
        color: white;
    }
    
    .button-icon {
        font-size: 1.1rem;
    }
    
    .button-label {
        font-size: 0.95rem;
    }
    
    /* Spinner */
    .spinner {
        width: 18px;
        height: 18px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }
    
    @keyframes spin {
        to { transform: rotate(360deg); }
    }
    
    /* Status indicator */
    .status-indicator {
        position: absolute;
        top: 50%;
        right: 10px;
        transform: translateY(-50%);
        width: 10px;
        height: 10px;
        border-radius: 50%;
    }
    
    .status-indicator.ready {
        background: #2ecc71;
        box-shadow: 0 0 0 0 currentColor;
        animation: pulse 2s infinite;
    }
    
    .status-indicator.error {
        background: #e74c3c;
        box-shadow: 0 0 0 0 currentColor;
        animation: pulse 2s infinite;
    }
    
    @keyframes pulse {
        0%, 100% { transform: translateY(-50%) scale(1); opacity: 1; }
        50% { transform: translateY(-50%) scale(1.2); opacity: 0.7; }
    }
</style>

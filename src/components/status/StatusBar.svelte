<script lang="ts">
    import { appStatus, files, template, isProcessing } from '../../stores/appStore';
    
    // Get validation status label
    const getValidationStatusLabel = (status: string) => {
        switch (status) {
            case 'NotStarted': return 'Non démarré';
            case 'InProgress': return 'En cours...';
            case 'Valid': return 'Valide';
            case 'Invalid': return 'Invalide';
            default: return status;
        }
    };
    
    // Get validation status icon
    const getValidationStatusIcon = (status: string) => {
        switch (status) {
            case 'NotStarted': return '⏳';
            case 'InProgress': return '🔄';
            case 'Valid': return '✅';
            case 'Invalid': return '❌';
            default: return '❓';
        }
    };
</script>

<footer class="status-bar">
    <div class="status-content">
        <!-- Files Status -->
        <div class="status-item">
            <span class="status-icon">📁</span>
            <span class="status-label">Fichiers</span>
            <span class="status-value">{$files.length}</span>
        </div>
        
        <!-- Template Status -->
        <div class="status-item">
            <span class="status-icon">📋</span>
            <span class="status-label">Modèle</span>
            <span class="status-value">
                {#if $template}
                    ✅
                {:else}
                    ❌
                {/if}
            </span>
        </div>
        
        <!-- Validation Status -->
        <div class="status-item">
            <span class="status-icon">{getValidationStatusIcon($appStatus.validationStatus)}</span>
            <span class="status-label">Validation</span>
            <span class="status-value">{getValidationStatusLabel($appStatus.validationStatus)}</span>
        </div>
        
        <!-- API Status -->
        <div class="status-item">
            <span class="status-icon">
                {#if $appStatus.apiConnected}
                    🟢
                {:else}
                    🔴
                {/if}
            </span>
            <span class="status-label">API</span>
            <span class="status-value">
                {#if $appStatus.apiConnected}
                    Connecté
                {:else}
                    Déconnecté
                {/if}
            </span>
        </div>
        
        <!-- Ready Status -->
        <div class="status-item ready-indicator">
            <span class="status-icon">
                {#if $appStatus.readyToSend}
                    ✅
                {:else}
                    ⏳
                {/if}
            </span>
            <span class="status-label">Prêt à envoyer</span>
            <span class="status-value">
                {#if $appStatus.readyToSend}
                    Oui
                {:else}
                    Non
                {/if}
            </span>
        </div>
        
        <!-- Processing Indicator -->
        {#if $isProcessing}
            <div class="processing-indicator">
                <span class="spinner"></span>
                <span>Traitement en cours...</span>
            </div>
        {/if}
    </div>
</footer>

<style>
    .status-bar {
        background: rgba(255, 255, 255, 0.95);
        backdrop-filter: blur(10px);
        padding: 1rem 2rem;
        border-top: 1px solid #eee;
        box-shadow: 0 -2px 10px rgba(0, 0, 0, 0.05);
    }
    
    .status-content {
        display: flex;
        justify-content: center;
        align-items: center;
        gap: 2rem;
        flex-wrap: wrap;
        max-width: 1200px;
        margin: 0 auto;
    }
    
    .status-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: white;
        border-radius: 8px;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
        transition: all 0.2s;
    }
    
    .status-item:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
    }
    
    .status-icon {
        font-size: 1.1rem;
    }
    
    .status-label {
        font-size: 0.85rem;
        color: #7f8c8d;
    }
    
    .status-value {
        font-weight: 600;
        color: #2c3e50;
        font-size: 0.9rem;
    }
    
    .ready-indicator {
        background: linear-gradient(135deg, #2ecc71 0%, #27ae60 100%);
        color: white;
    }
    
    .ready-indicator .status-label,
    .ready-indicator .status-value {
        color: white;
    }
    
    .processing-indicator {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        border-radius: 8px;
        font-size: 0.9rem;
        animation: pulse 2s infinite;
    }
    
    .spinner {
        width: 16px;
        height: 16px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }
    
    @keyframes spin {
        to { transform: rotate(360deg); }
    }
    
    @keyframes pulse {
        0%, 100% { opacity: 1; }
        50% { opacity: 0.7; }
    }
    
    @media (max-width: 768px) {
        .status-bar {
            padding: 0.75rem 1rem;
        }
        
        .status-content {
            gap: 1rem;
        }
        
        .status-item {
            padding: 0.5rem 0.75rem;
        }
        
        .status-icon {
            font-size: 1rem;
        }
        
        .status-label {
            font-size: 0.8rem;
        }
        
        .status-value {
            font-size: 0.85rem;
        }
    }
</style>

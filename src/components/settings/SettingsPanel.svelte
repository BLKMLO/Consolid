<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import { apiKey, setApiKey, clearApiKey } from '../../stores/appStore';
    import { invoke } from '@tauri-apps/api/tauri';
    
    const dispatch = createEventDispatcher();
    
    let apiKeyInput = '';
    let isTesting = false;
    let testResult: 'success' | 'error' | null = null;
    
    // Initialize with current API key (masked)
    $: {
        apiKeyInput = $apiKey ? '••••••••' : '';
    }
    
    // Handle API key change
    const handleApiKeyChange = async () => {
        if (apiKeyInput.trim() === '') {
            clearApiKey();
            dispatch('apiKeySet');
            return;
        }
        
        setApiKey(apiKeyInput.trim());
        dispatch('apiKeySet');
    };
    
    // Test API connection
    const testConnection = async () => {
        if (!$apiKey) {
            testResult = 'error';
            return;
        }
        
        isTesting = true;
        testResult = null;
        
        try {
            const connected = await invoke('test_api_connection');
            testResult = connected ? 'success' : 'error';
        } catch (error) {
            testResult = 'error';
        } finally {
            isTesting = false;
        }
    };
    
    // Clear API key
    const handleClearApiKey = () => {
        apiKeyInput = '';
        clearApiKey();
        testResult = null;
        dispatch('apiKeySet');
    };
</script>

<div class="settings-panel">
    <div class="settings-header">
        <h2>⚙️ Paramètres</h2>
        <button class="btn-close" on:click={() => dispatch('close')}>
            ×
        </button>
    </div>
    
    <div class="settings-content">
        <!-- API Key Section -->
        <section class="setting-section">
            <h3>🔑 Clé API Mistral</h3>
            <p class="setting-description">
                Configurez votre clé API Mistral pour activer l'analyse IA.
                <a href="https://mistral.ai/" target="_blank" rel="noopener">Obtenir une clé API</a>
            </p>
            
            <div class="input-group">
                <input 
                    type="password"
                    bind:value={apiKeyInput}
                    placeholder="Entrez votre clé API Mistral..."
                    class="input-field"
                    on:change={handleApiKeyChange}
                />
                <button 
                    class="btn-action"
                    on:click={handleApiKeyChange}
                    disabled={!apiKeyInput.trim()}
                    title="Enregistrer"
                >
                    ✓
                </button>
            </div>
            
            {#if $apiKey}
                <div class="api-key-actions">
                    <button class="btn-test" on:click={testConnection} disabled={isTesting}>
                        {#if isTesting}
                            <span class="spinner"></span> Test en cours...
                        {:else}
                            Tester la connexion
                        {/if}
                    </button>
                    <button class="btn-clear" on:click={handleClearApiKey}>
                        Effacer
                    </button>
                </div>
                
                {#if testResult === 'success'}
                    <div class="test-result success">
                        ✓ Connexion réussie!
                    </div>
                {:else if testResult === 'error'}
                    <div class="test-result error">
                        ✗ Échec de la connexion. Vérifiez votre clé API.
                    </div>
                {/if}
            {/if}
        </section>
        
        <!-- Model Settings -->
        <section class="setting-section">
            <h3>🤖 Paramètres du modèle</h3>
            <p class="setting-description">
                Configurez les paramètres pour l'analyse IA.
            </p>
            
            <div class="model-settings">
                <div class="setting-item">
                    <label>Modèle</label>
                    <select class="input-field">
                        <option value="mistral-tiny">Mistral Tiny</option>
                        <option value="mistral-small">Mistral Small</option>
                        <option value="mistral-medium">Mistral Medium</option>
                        <option value="mistral-large">Mistral Large</option>
                    </select>
                </div>
                
                <div class="setting-item">
                    <label>Température</label>
                    <input type="range" min="0" max="1" step="0.1" value="0.7" class="slider">
                    <span class="slider-value">0.7</span>
                </div>
                
                <div class="setting-item">
                    <label>Tokens maximum</label>
                    <input type="number" min="100" max="32000" value="4096" class="input-field">
                </div>
            </div>
        </section>
        
        <!-- Anonymization Settings -->
        <section class="setting-section">
            <h3>🔒 Anonymisation</h3>
            <p class="setting-description">
                Paramètres pour l'anonymisation des données sensibles.
            </p>
            
            <div class="anonymization-settings">
                <label class="checkbox-label">
                    <input type="checkbox" checked={true}>
                    <span>Anonymiser automatiquement les emails</span>
                </label>
                <label class="checkbox-label">
                    <input type="checkbox" checked={true}>
                    <span>Anonymiser les numéros de téléphone</span>
                </label>
                <label class="checkbox-label">
                    <input type="checkbox" checked={true}>
                    <span>Anonymiser les SIREN/SIRET</span>
                </label>
                <label class="checkbox-label">
                    <input type="checkbox" checked={true}>
                    <span>Anonymiser les adresses</span>
                </label>
                <label class="checkbox-label">
                    <input type="checkbox" checked={true}>
                    <span>Anonymiser les noms et prénoms</span>
                </label>
            </div>
        </section>
        
        <!-- About Section -->
        <section class="setting-section about">
            <h3>ℹ️ À propos</h3>
            <p>Consolid Audit v0.1.0</p>
            <p>Outil d'audit et de consolidation comptable avec anonymisation locale et analyse IA.</p>
            <p class="copyright">© 2024 BLKMLO. Tous droits réservés.</p>
        </section>
    </div>
</div>

<style>
    .settings-panel {
        padding: 1.5rem;
        background: white;
        height: 100%;
        overflow-y: auto;
    }
    
    .settings-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1.5rem;
        padding-bottom: 1rem;
        border-bottom: 1px solid #eee;
    }
    
    .settings-header h2 {
        margin: 0;
        color: #2c3e50;
        font-size: 1.3rem;
    }
    
    .btn-close {
        background: none;
        border: none;
        font-size: 1.5rem;
        cursor: pointer;
        color: #95a5a6;
        padding: 0.25rem;
        line-height: 1;
        transition: color 0.2s;
    }
    
    .btn-close:hover {
        color: #7f8c8d;
    }
    
    .settings-content {
        display: flex;
        flex-direction: column;
        gap: 2rem;
    }
    
    .setting-section {
        animation: fadeIn 0.3s ease;
    }
    
    .setting-section h3 {
        margin: 0 0 0.5rem;
        color: #2c3e50;
        font-size: 1.1rem;
    }
    
    .setting-description {
        margin: 0 0 1rem;
        color: #7f8c8d;
        font-size: 0.9rem;
        line-height: 1.5;
    }
    
    .setting-description a {
        color: #667eea;
        text-decoration: none;
    }
    
    .setting-description a:hover {
        text-decoration: underline;
    }
    
    .input-group {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }
    
    .input-field {
        flex: 1;
        padding: 0.75rem 1rem;
        border: 1px solid #ddd;
        border-radius: 6px;
        font-size: 0.95rem;
        transition: all 0.2s;
    }
    
    .input-field:focus {
        outline: none;
        border-color: #667eea;
        box-shadow: 0 0 0 2px rgba(102, 126, 234, 0.1);
    }
    
    .btn-action {
        background: #667eea;
        color: white;
        border: none;
        padding: 0.75rem 1rem;
        border-radius: 6px;
        cursor: pointer;
        font-size: 1rem;
        transition: all 0.2s;
        width: 40px;
    }
    
    .btn-action:hover:not(:disabled) {
        background: #5568d3;
    }
    
    .btn-action:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    
    .api-key-actions {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }
    
    .btn-test {
        background: #4facfe;
        color: white;
        border: none;
        padding: 0.5rem 1rem;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.9rem;
        transition: all 0.2s;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    
    .btn-test:hover:not(:disabled) {
        background: #3a9ae8;
    }
    
    .btn-test:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    
    .btn-clear {
        background: #e74c3c;
        color: white;
        border: none;
        padding: 0.5rem 1rem;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.9rem;
        transition: all 0.2s;
    }
    
    .btn-clear:hover {
        background: #c0392b;
    }
    
    .test-result {
        padding: 0.5rem 1rem;
        border-radius: 6px;
        font-size: 0.9rem;
        margin-top: 0.5rem;
    }
    
    .test-result.success {
        background: #efe;
        border: 1px solid #cfc;
        color: #090;
    }
    
    .test-result.error {
        background: #fee;
        border: 1px solid #fcc;
        color: #c00;
    }
    
    .model-settings {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    
    .setting-item {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    
    .setting-item label {
        font-size: 0.9rem;
        color: #2c3e50;
        font-weight: 500;
    }
    
    .slider {
        width: 100%;
        height: 6px;
        border-radius: 3px;
        background: #f0f0f0;
        outline: none;
        -webkit-appearance: none;
    }
    
    .slider::-webkit-slider-thumb {
        -webkit-appearance: none;
        appearance: none;
        width: 18px;
        height: 18px;
        border-radius: 50%;
        background: #667eea;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .slider::-webkit-slider-thumb:hover {
        transform: scale(1.1);
        background: #5568d3;
    }
    
    .slider::-moz-range-thumb {
        width: 18px;
        height: 18px;
        border-radius: 50%;
        background: #667eea;
        cursor: pointer;
        border: none;
    }
    
    .slider-value {
        font-size: 0.85rem;
        color: #7f8c8d;
        text-align: center;
    }
    
    .anonymization-settings {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }
    
    .checkbox-label {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        cursor: pointer;
        padding: 0.5rem;
        border-radius: 6px;
        transition: background 0.2s;
    }
    
    .checkbox-label:hover {
        background: #f8f9fa;
    }
    
    .checkbox-label input[type="checkbox"] {
        width: 18px;
        height: 18px;
        accent-color: #667eea;
        cursor: pointer;
    }
    
    .checkbox-label span {
        font-size: 0.9rem;
        color: #2c3e50;
    }
    
    .about {
        margin-top: auto;
    }
    
    .about p {
        margin: 0.25rem 0;
        color: #7f8c8d;
        font-size: 0.9rem;
    }
    
    .copyright {
        font-size: 0.8rem !important;
        color: #95a5a6 !important;
        margin-top: 0.5rem !important;
    }
    
    .spinner {
        width: 14px;
        height: 14px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }
    
    @keyframes spin {
        to { transform: rotate(360deg); }
    }
    
    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
</style>

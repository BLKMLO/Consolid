<script lang="ts">
    import { onMount } from 'svelte';
    import FileDropZone from './components/file/FileDropZone.svelte';
    import FileList from './components/file/FileList.svelte';
    import SettingsPanel from './components/settings/SettingsPanel.svelte';
    import StatusBar from './components/status/StatusBar.svelte';
    import ActionButton from './components/common/ActionButton.svelte';
    import { files, template, apiKey, appStatus, isProcessing } from './stores/appStore';
    import { invoke } from '@tauri-apps/api/tauri';
    import { save } from '@tauri-apps/api/dialog';
    import { writeTextFile } from '@tauri-apps/api/fs';
    
    let showSettings = false;
    let errorMessage: string | null = null;
    let successMessage: string | null = null;
    
    // Handle file drop
    const handleFileDrop = async (droppedFiles: string[]) => {
        try {
            errorMessage = null;
            const result = await invoke('handle_file_drop', { paths: droppedFiles });
            files.set(result as any);
        } catch (error) {
            errorMessage = `Erreur lors de l'ajout des fichiers: ${error}`;
        }
    };
    
    // Process files
    const processFiles = async () => {
        if ($files.length === 0) {
            errorMessage = "Veuillez ajouter des fichiers d'abord";
            return;
        }
        
        if (!$template) {
            errorMessage = "Veuillez sélectionner un modèle (template) d'abord";
            return;
        }
        
        try {
            isProcessing.set(true);
            errorMessage = null;
            successMessage = null;
            
            const filePaths = $files.map(f => f.path);
            
            // Process files (validate and anonymize)
            const result = await invoke('process_files', {
                filePaths,
                templatePath: $template.path
            });
            
            // Update app status
            await updateAppStatus();
            
            successMessage = "Fichiers traités avec succès! Prêt pour l'envoi à Mistral.";
            
        } catch (error) {
            errorMessage = `Erreur lors du traitement: ${error}`;
        } finally {
            isProcessing.set(false);
        }
    };
    
    // Send to Mistral
    const sendToMistral = async () => {
        try {
            if (!$apiKey) {
                errorMessage = "Veuillez configurer votre clé API Mistral d'abord";
                return;
            }
            
            if (!$appStatus.ready_to_send) {
                errorMessage = "Tous les fichiers ne sont pas valides ou l'API n'est pas connectée";
                return;
            }
            
            isProcessing.set(true);
            errorMessage = null;
            successMessage = null;
            
            // Get processed files
            const processedFiles = $files.filter(f => f.isValid);
            
            // Send to Mistral
            const result = await invoke('send_to_mistral', {
                anonymizedFiles: processedFiles,
                templateContent: $template?.content || '',
                apiKey: $apiKey
            });
            
            // Save the result
            if (result && (result as any).filledTemplate) {
                const savePath = await save({
                    filters: [
                        { name: 'Fichier texte', extensions: ['txt'] },
                        { name: 'CSV', extensions: ['csv'] },
                        { name: 'Tous les fichiers', extensions: ['*'] }
                    ]
                });
                
                if (savePath) {
                    await writeTextFile(savePath, (result as any).filledTemplate);
                    successMessage = `Modèle rempli sauvegardé avec succès! Tokens utilisés: ${(result as any).tokensUsed || 0}`;
                }
            }
            
            // Update app status
            await updateAppStatus();
            
        } catch (error) {
            errorMessage = `Erreur lors de l'envoi à Mistral: ${error}`;
        } finally {
            isProcessing.set(false);
        }
    };
    
    // Update app status
    const updateAppStatus = async () => {
        try {
            const status = await invoke('get_app_status');
            appStatus.set(status as any);
        } catch (error) {
            console.error('Failed to get app status:', error);
        }
    };
    
    // Test API connection
    const testApiConnection = async () => {
        if (!$apiKey) {
            errorMessage = "Veuillez configurer votre clé API Mistral d'abord";
            return;
        }
        
        try {
            isProcessing.set(true);
            const connected = await invoke('test_api_connection');
            
            if (connected) {
                successMessage = "Connexion à Mistral réussie!";
            } else {
                errorMessage = "Échec de la connexion à Mistral. Vérifiez votre clé API.";
            }
            
            await updateAppStatus();
        } catch (error) {
            errorMessage = `Erreur de connexion: ${error}`;
        } finally {
            isProcessing.set(false);
        }
    };
    
    // Clear messages
    const clearMessages = () => {
        errorMessage = null;
        successMessage = null;
    };
    
    // Initialize on mount
    onMount(async () => {
        await updateAppStatus();
    });
</script>

<svelte:head>
    <title>Consolid Audit - Audit Comptable avec IA</title>
</svelte:head>

<div class="app-container">
    <!-- Header -->
    <header class="app-header">
        <div class="header-content">
            <h1>📊 Consolid Audit</h1>
            <p class="subtitle">Outil d'audit et de consolidation comptable avec anonymisation locale</p>
        </div>
        <div class="header-actions">
            <button 
                class="btn btn-secondary" 
                on:click={() => showSettings = !showSettings}
            >
                ⚙️ Paramètres
            </button>
        </div>
    </header>

    <!-- Main Content -->
    <main class="main-content">
        <!-- Settings Panel (Slide-in) -->
        <div class="settings-panel" class:open={showSettings}>
            <SettingsPanel 
                on:close={() => showSettings = false}
                on:apiKeySet={testApiConnection}
            />
        </div>
        
        <!-- File Drop Zone -->
        <section class="file-section">
            <FileDropZone on:filesDropped={handleFileDrop} />
            
            <!-- File List -->
            <FileList />
        </section>
        
        <!-- Messages -->
        {#if errorMessage}
            <div class="message error">
                <span>❌ {errorMessage}</span>
                <button class="btn-icon" on:click={clearMessages}>×</button>
            </div>
        {/if}
        
        {#if successMessage}
            <div class="message success">
                <span>✅ {successMessage}</span>
                <button class="btn-icon" on:click={clearMessages}>×</button>
            </div>
        {/if}
        
        <!-- Action Buttons -->
        <div class="action-bar">
            <ActionButton 
                label="Traiter les fichiers"
                icon="🔄"
                on:click={processFiles}
                disabled={$files.length === 0 || !$template}
                loading={$isProcessing}
                variant="primary"
            />
            
            <ActionButton 
                label="Envoyer à Mistral"
                icon="🚀"
                on:click={sendToMistral}
                disabled={!$appStatus.ready_to_send || $isProcessing}
                loading={$isProcessing}
                variant="success"
                status={$appStatus.ready_to_send ? 'ready' : 'disabled'}
            />
        </div>
    </main>
    
    <!-- Status Bar -->
    <StatusBar />
</div>

<style>
    .app-container {
        display: flex;
        flex-direction: column;
        min-height: 100vh;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    }
    
    .app-header {
        background: rgba(255, 255, 255, 0.95);
        backdrop-filter: blur(10px);
        padding: 1.5rem 2rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
        position: sticky;
        top: 0;
        z-index: 100;
    }
    
    .header-content h1 {
        margin: 0;
        color: #2c3e50;
        font-size: 1.8rem;
        font-weight: 700;
    }
    
    .subtitle {
        margin: 0.25rem 0 0;
        color: #7f8c8d;
        font-size: 0.9rem;
    }
    
    .header-actions {
        display: flex;
        gap: 0.75rem;
    }
    
    .main-content {
        flex: 1;
        padding: 2rem;
        max-width: 1400px;
        margin: 0 auto;
        width: 100%;
    }
    
    .file-section {
        margin-bottom: 2rem;
    }
    
    .action-bar {
        display: flex;
        gap: 1rem;
        justify-content: center;
        margin-top: 2rem;
        flex-wrap: wrap;
    }
    
    .message {
        padding: 1rem 1.5rem;
        border-radius: 8px;
        margin: 1rem 0;
        display: flex;
        justify-content: space-between;
        align-items: center;
        animation: slideIn 0.3s ease;
    }
    
    .message.error {
        background: #fee;
        border: 1px solid #fcc;
        color: #c00;
    }
    
    .message.success {
        background: #efe;
        border: 1px solid #cfc;
        color: #090;
    }
    
    .btn-icon {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 1.2rem;
        padding: 0 0 0 1rem;
        opacity: 0.7;
        transition: opacity 0.2s;
    }
    
    .btn-icon:hover {
        opacity: 1;
    }
    
    .settings-panel {
        position: fixed;
        top: 0;
        right: -400px;
        width: 400px;
        height: 100vh;
        background: white;
        box-shadow: -2px 0 10px rgba(0, 0, 0, 0.1);
        transition: right 0.3s ease;
        z-index: 1000;
        overflow-y: auto;
    }
    
    .settings-panel.open {
        right: 0;
    }
    
    @keyframes slideIn {
        from {
            opacity: 0;
            transform: translateY(-10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
    
    @media (max-width: 768px) {
        .app-header {
            flex-direction: column;
            gap: 1rem;
            text-align: center;
        }
        
        .header-actions {
            width: 100%;
            justify-content: center;
        }
        
        .main-content {
            padding: 1rem;
        }
        
        .settings-panel {
            width: 100%;
            right: -100%;
        }
    }
</style>

<script lang="ts">
    import { files, template, removeFile, clearFiles } from '../../stores/appStore';
    import { invoke } from '@tauri-apps/api/tauri';
    
    // Remove a file
    const handleRemoveFile = (path: string) => {
        removeFile(path);
    };
    
    // Clear all files
    const handleClearAll = () => {
        clearFiles();
    };
    
    // Get file type icon
    const getFileTypeIcon = (fileType: string) => {
        switch (fileType) {
            case 'Csv': return '📊';
            case 'Excel': return '📈';
            case 'Pdf': return '📄';
            case 'Text': return '📝';
            default: return '📁';
        }
    };
    
    // Get file type label
    const getFileTypeLabel = (fileType: string) => {
        switch (fileType) {
            case 'Csv': return 'CSV';
            case 'Excel': return 'Excel';
            case 'Pdf': return 'PDF';
            case 'Text': return 'Texte';
            default: return 'Inconnu';
        }
    };
    
    // Format file size
    const formatFileSize = (bytes: number) => {
        if (bytes < 1024) return `${bytes} octets`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} Ko`;
        return `${(bytes / (1024 * 1024)).toFixed(2)} Mo`;
    };
</script>

{#if $files.length > 0 || $template}
    <div class="file-list-container">
        <!-- Template Section -->
        {#if $template}
            <div class="template-card">
                <div class="file-item">
                    <div class="file-icon">📋</div>
                    <div class="file-info">
                        <div class="file-name">{$template.name}</div>
                        <div class="file-meta">
                            <span class="file-type">Modèle</span>
                            <span class="file-size">{formatFileSize($template.content.length)}</span>
                        </div>
                    </div>
                    <button 
                        class="btn-remove"
                        on:click={() => clearFiles()}
                        title="Retirer le modèle"
                    >
                        ×
                    </button>
                </div>
            </div>
        {/if}
        
        <!-- Files Section -->
        {#if $files.length > 0}
            <div class="files-header">
                <h3>Fichiers à traiter ({$files.length})</h3>
                <button class="btn-clear" on:click={handleClearAll}>
                    Tout supprimer
                </button>
            </div>
            
            <div class="file-list">
                {#each $files as file (file.path)}
                    <div class="file-item" class:invalid={!file.isValid}>
                        <div class="file-icon">{getFileTypeIcon(file.fileType)}</div>
                        <div class="file-info">
                            <div class="file-name">{file.name}</div>
                            <div class="file-meta">
                                <span class="file-type">{getFileTypeLabel(file.fileType)}</span>
                                <span class="file-size">{formatFileSize(file.size)}</span>
                            </div>
                        </div>
                        <div class="file-status">
                            {#if file.isValid}
                                <span class="status-badge valid" title="Valide">✓</span>
                            {:else}
                                <span class="status-badge invalid" title={file.error || 'Invalide'}>✗</span>
                            {/if}
                        </div>
                        <button 
                            class="btn-remove"
                            on:click={() => handleRemoveFile(file.path)}
                            title="Retirer le fichier"
                        >
                            ×
                        </button>
                    </div>
                {/each}
            </div>
        {/if}
    </div>
{/if}

<style>
    .file-list-container {
        margin-top: 1.5rem;
    }
    
    .template-card {
        background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
        border-radius: 12px;
        padding: 0.5rem;
        margin-bottom: 1.5rem;
        box-shadow: 0 4px 6px rgba(79, 172, 254, 0.1);
    }
    
    .files-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }
    
    .files-header h3 {
        margin: 0;
        color: #2c3e50;
        font-size: 1.1rem;
    }
    
    .btn-clear {
        background: none;
        border: none;
        color: #e74c3c;
        cursor: pointer;
        font-size: 0.9rem;
        padding: 0.25rem 0.5rem;
        border-radius: 4px;
        transition: all 0.2s;
    }
    
    .btn-clear:hover {
        background: #fee;
        color: #c00;
    }
    
    .file-list {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }
    
    .file-item {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1rem;
        background: white;
        border-radius: 8px;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
        transition: all 0.2s ease;
        border: 1px solid #eee;
    }
    
    .file-item:hover {
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
        transform: translateY(-2px);
    }
    
    .file-item.invalid {
        border-color: #e74c3c;
        background: #fee;
    }
    
    .file-icon {
        font-size: 1.5rem;
        width: 40px;
        text-align: center;
    }
    
    .file-info {
        flex: 1;
        min-width: 0;
    }
    
    .file-name {
        font-weight: 600;
        color: #2c3e50;
        margin-bottom: 0.25rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    
    .file-meta {
        display: flex;
        gap: 1rem;
        font-size: 0.85rem;
        color: #7f8c8d;
    }
    
    .file-type {
        background: #f0f0f0;
        padding: 0.125rem 0.5rem;
        border-radius: 4px;
        font-size: 0.75rem;
    }
    
    .file-size {
        font-size: 0.8rem;
    }
    
    .file-status {
        margin-right: 0.5rem;
    }
    
    .status-badge {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 24px;
        height: 24px;
        border-radius: 50%;
        font-size: 0.75rem;
        font-weight: bold;
    }
    
    .status-badge.valid {
        background: #2ecc71;
        color: white;
    }
    
    .status-badge.invalid {
        background: #e74c3c;
        color: white;
    }
    
    .btn-remove {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 1.2rem;
        color: #95a5a6;
        padding: 0.25rem;
        border-radius: 4px;
        transition: all 0.2s;
        line-height: 1;
    }
    
    .btn-remove:hover {
        background: #eee;
        color: #7f8c8d;
    }
    
    @media (max-width: 768px) {
        .file-item {
            padding: 0.75rem;
        }
        
        .file-icon {
            font-size: 1.25rem;
            width: 32px;
        }
        
        .file-name {
            font-size: 0.95rem;
        }
        
        .file-meta {
            font-size: 0.8rem;
        }
    }
</style>

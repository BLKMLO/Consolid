<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import { files } from '../../stores/appStore';
    
    const dispatch = createEventDispatcher();
    
    let isDragging = false;
    let dropZoneRef: HTMLDivElement;
    
    // Handle drag events
    const handleDragOver = (e: DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        isDragging = true;
    };
    
    const handleDragLeave = (e: DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        isDragging = false;
    };
    
    const handleDragEnter = (e: DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        isDragging = true;
    };
    
    const handleDrop = (e: DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        isDragging = false;
        
        const dt = e.dataTransfer;
        if (dt && dt.files && dt.files.length > 0) {
            const filePaths: string[] = [];
            
            for (let i = 0; i < dt.files.length; i++) {
                // For browser, we get File objects
                // For Tauri, we need to handle file paths differently
                // This will be handled by the backend
                const file = dt.files[i];
                console.log('Dropped file:', file.name, file.size);
                
                // In Tauri, we'll use the tauri API to get the actual file paths
                // For now, we'll simulate with the file names
                // The actual implementation will use tauri's dialog API
            }
            
            // For Tauri, we need to use the file dialog API
            // This is a workaround since we can't directly access file paths from drag and drop
            // We'll use a custom event that the backend can handle
            
            // For now, we'll just emit the event with the file names
            // The backend will handle the actual file paths
            dispatch('filesDropped', Array.from(dt.files).map(f => f.name));
        }
    };
    
    // Handle click to open file dialog
    const handleClick = async () => {
        try {
            // Use Tauri's dialog API to open file picker
            const { open } = await import('@tauri-apps/api/dialog');
            const selected = await open({
                multiple: true,
                filters: [
                    { name: 'Fichiers comptables', extensions: ['csv', 'xlsx', 'xls', 'ods', 'txt'] },
                    { name: 'Tous les fichiers', extensions: ['*'] }
                ]
            });
            
            if (selected && Array.isArray(selected)) {
                dispatch('filesDropped', selected);
            } else if (selected) {
                dispatch('filesDropped', [selected]);
            }
        } catch (error) {
            console.error('Failed to open file dialog:', error);
        }
    };
    
    // Handle template selection
    const handleTemplateClick = async () => {
        try {
            const { open } = await import('@tauri-apps/api/dialog');
            const selected = await open({
                multiple: false,
                filters: [
                    { name: 'Modèles', extensions: ['xlsx', 'xls', 'csv', 'txt', 'json'] },
                    { name: 'Tous les fichiers', extensions: ['*'] }
                ]
            });
            
            if (selected) {
                // Read the template file
                const { readTextFile } = await import('@tauri-apps/api/fs');
                const content = await readTextFile(selected);
                
                // Set the template in the store
                const { setTemplate } = await import('../../stores/appStore');
                setTemplate({
                    path: selected,
                    name: selected.split('/').pop() || 'template',
                    content
                });
            }
        } catch (error) {
            console.error('Failed to open template dialog:', error);
        }
    };
</script>

<div 
    class="drop-zone"
    class:dragging={isDragging}
    bind:this={dropZoneRef}
    on:dragover={handleDragOver}
    on:dragleave={handleDragLeave}
    on:dragenter={handleDragEnter}
    on:drop={handleDrop}
    on:click={handleClick}
>
    <div class="drop-zone-content">
        <div class="drop-zone-icon">📁</div>
        <h3>Glissez-déposez vos fichiers ici</h3>
        <p>ou cliquez pour sélectionner des fichiers</p>
        <p class="supported-formats">
            Formats supportés: CSV, Excel (XLSX, XLS, ODS), TXT
        </p>
        
        {#if $files.length === 0}
            <button class="btn-select" on:click|stopPropagation={handleClick}>
                Sélectionner des fichiers
            </button>
        {/if}
    </div>
    
    <!-- Template selection -->
    <div class="template-section">
        <button class="btn-template" on:click|stopPropagation={handleTemplateClick}>
            {#if $files.length > 0}
                ➕ Ajouter un modèle
            {:else}
                📄 Sélectionner un modèle (template)
            {/if}
        </button>
    </div>
</div>

<style>
    .drop-zone {
        border: 2px dashed #ccc;
        border-radius: 12px;
        padding: 3rem;
        text-align: center;
        background: white;
        cursor: pointer;
        transition: all 0.3s ease;
        position: relative;
        overflow: hidden;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.05);
    }
    
    .drop-zone:hover {
        border-color: #667eea;
        box-shadow: 0 6px 12px rgba(102, 126, 234, 0.15);
    }
    
    .drop-zone.dragging {
        border-color: #667eea;
        background: linear-gradient(135deg, rgba(102, 126, 234, 0.05) 0%, rgba(118, 75, 162, 0.05) 100%);
        box-shadow: 0 8px 16px rgba(102, 126, 234, 0.2);
    }
    
    .drop-zone-content {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;
    }
    
    .drop-zone-icon {
        font-size: 3rem;
        opacity: 0.7;
        transition: transform 0.3s ease;
    }
    
    .drop-zone.dragging .drop-zone-icon {
        transform: scale(1.1);
        opacity: 1;
    }
    
    .drop-zone h3 {
        margin: 0;
        color: #2c3e50;
        font-size: 1.2rem;
    }
    
    .drop-zone p {
        margin: 0;
        color: #7f8c8d;
        font-size: 0.95rem;
    }
    
    .supported-formats {
        font-size: 0.85rem !important;
        color: #95a5a6 !important;
    }
    
    .btn-select {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        border: none;
        padding: 0.75rem 1.5rem;
        border-radius: 8px;
        font-size: 1rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s ease;
        box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
        margin-top: 1rem;
    }
    
    .btn-select:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 10px rgba(0, 0, 0, 0.15);
    }
    
    .template-section {
        margin-top: 2rem;
        padding-top: 2rem;
        border-top: 1px solid #eee;
        text-align: center;
    }
    
    .btn-template {
        background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
        color: white;
        border: none;
        padding: 0.75rem 1.5rem;
        border-radius: 8px;
        font-size: 0.95rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s ease;
        box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
    }
    
    .btn-template:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 10px rgba(79, 172, 254, 0.2);
    }
    
    @media (max-width: 768px) {
        .drop-zone {
            padding: 2rem 1.5rem;
        }
        
        .drop-zone-icon {
            font-size: 2.5rem;
        }
        
        .drop-zone h3 {
            font-size: 1.1rem;
        }
    }
</style>

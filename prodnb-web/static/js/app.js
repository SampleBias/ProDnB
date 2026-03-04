// ProDnB Web UI - Main Application Logic

class ProDnBApp {
    constructor() {
        this.uploadedFilePath = null;
        this.proteinInfo = null;
        
        this.initElements();
        this.initEventListeners();
    }

    initElements() {
        // File upload elements
        this.uploadArea = document.getElementById('uploadArea');
        this.fileInput = document.getElementById('fileInput');
        this.fileInfo = document.getElementById('fileInfo');
        this.fileName = document.getElementById('fileName');
        this.fileStats = document.getElementById('fileStats');
        this.removeFileBtn = document.getElementById('removeFile');

        // Generate elements
        this.generateBtn = document.getElementById('generateBtn');
        this.btnText = this.generateBtn.querySelector('.btn-text');
        this.btnLoading = this.generateBtn.querySelector('.btn-loading');

        // Output elements
        this.strudelCode = document.getElementById('strudelCode');
        this.copyBtn = document.getElementById('copyBtn');
        this.clearBtn = document.getElementById('clearBtn');

        // Status message
        this.statusMessage = document.getElementById('statusMessage');
    }

    initEventListeners() {
        // Upload area click
        this.uploadArea.addEventListener('click', () => {
            this.fileInput.click();
        });

        // File input change
        this.fileInput.addEventListener('change', (e) => {
            if (e.target.files.length > 0) {
                this.handleFileUpload(e.target.files[0]);
            }
        });

        // Drag and drop
        this.uploadArea.addEventListener('dragover', (e) => {
            e.preventDefault();
            this.uploadArea.classList.add('dragover');
        });

        this.uploadArea.addEventListener('dragleave', () => {
            this.uploadArea.classList.remove('dragover');
        });

        this.uploadArea.addEventListener('drop', (e) => {
            e.preventDefault();
            this.uploadArea.classList.remove('dragover');
            
            if (e.dataTransfer.files.length > 0) {
                this.handleFileUpload(e.dataTransfer.files[0]);
            }
        });

        // Remove file
        this.removeFileBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            this.clearFileUpload();
        });

        // Generate Strudel code
        this.generateBtn.addEventListener('click', () => {
            this.generateStrudel();
        });

        // Copy code
        this.copyBtn.addEventListener('click', () => {
            this.copyCode();
        });

        // Clear code
        this.clearBtn.addEventListener('click', () => {
            this.clearOutput();
        });
    }

    async handleFileUpload(file) {
        // Validate file type
        const validExtensions = ['.pdb', '.ent', '.cif'];
        const fileName = file.name.toLowerCase();
        const isValid = validExtensions.some(ext => fileName.endsWith(ext));

        if (!isValid) {
            this.showStatus('Please upload a PDB, ENT, or CIF file', 'error');
            return;
        }

        // Validate file size (10MB)
        if (file.size > 10 * 1024 * 1024) {
            this.showStatus('File too large. Maximum size is 10MB', 'error');
            return;
        }

        // Show file info with loading state
        this.fileName.textContent = `${file.name} (uploading...)`;
        this.fileInfo.classList.remove('hidden');
        this.uploadArea.classList.add('hidden');

        try {
            // Upload file to server
            const formData = new FormData();
            formData.append('pdb_file', file);

            const response = await fetch('/api/upload', {
                method: 'POST',
                body: formData
            });

            const result = await response.json();

            if (!response.ok || !result.success) {
                throw new Error(result.error || 'Upload failed');
            }

            // Store file path and protein info
            this.uploadedFilePath = result.file_path;
            this.proteinInfo = {
                chain_count: result.chain_count,
                residue_count: result.residue_count,
                atom_count: result.atom_count
            };

            // Update UI
            this.fileName.textContent = file.name;
            this.fileStats.textContent = `Chains: ${result.chain_count} | Residues: ${result.residue_count} | Atoms: ${result.atom_count}`;
            this.generateBtn.disabled = false;
            
            this.showStatus('PDB file uploaded successfully!', 'success');

        } catch (error) {
            console.error('Upload error:', error);
            this.clearFileUpload();
            this.showStatus(error.message || 'Failed to upload file', 'error');
        }
    }

    clearFileUpload() {
        this.uploadedFilePath = null;
        this.proteinInfo = null;
        this.fileInput.value = '';
        this.fileInfo.classList.add('hidden');
        this.uploadArea.classList.remove('hidden');
        this.generateBtn.disabled = true;
    }

    async generateStrudel() {
        if (!this.uploadedFilePath) {
            this.showStatus('Please upload a PDB file first', 'error');
            return;
        }

        // Update UI to loading state
        this.setGeneratingState(true);
        this.strudelCode.textContent = '// Generating Strudel code...';
        this.strudelCode.style.color = '#666666';

        try {
            const response = await fetch('/api/generate', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    file_path: this.uploadedFilePath
                })
            });

            const result = await response.json();

            if (!response.ok || !result.success) {
                throw new Error(result.error || 'Generation failed');
            }

            // Display generated code
            this.strudelCode.textContent = result.code;
            this.strudelCode.style.color = '';
            this.copyBtn.disabled = false;
            this.clearBtn.classList.remove('hidden');

            this.showStatus('Strudel code generated successfully!', 'success');

        } catch (error) {
            console.error('Generation error:', error);
            this.strudelCode.textContent = `// Error: ${error.message}`;
            this.strudelCode.style.color = '#b91c1c';
            this.showStatus(error.message || 'Failed to generate code', 'error');
        } finally {
            this.setGeneratingState(false);
        }
    }

    setGeneratingState(isGenerating) {
        this.generateBtn.disabled = isGenerating;
        
        if (isGenerating) {
            this.btnText.classList.add('hidden');
            this.btnLoading.classList.remove('hidden');
        } else {
            this.btnText.classList.remove('hidden');
            this.btnLoading.classList.add('hidden');
        }
    }

    copyCode() {
        const code = this.strudelCode.textContent;
        
        navigator.clipboard.writeText(code).then(() => {
            this.showStatus('Code copied to clipboard!', 'success');
            
            // Visual feedback on button
            const originalText = this.copyBtn.querySelector('span').textContent;
            this.copyBtn.querySelector('span').textContent = 'Copied!';
            setTimeout(() => {
                this.copyBtn.querySelector('span').textContent = originalText;
            }, 2000);
        }).catch(err => {
            console.error('Copy failed:', err);
            this.showStatus('Failed to copy code', 'error');
        });
    }

    clearOutput() {
        this.strudelCode.textContent = '// Your Strudel code will appear here...\n// Upload a PDB file and click "Generate" to start';
        this.copyBtn.disabled = true;
        this.clearBtn.classList.add('hidden');
    }

    showStatus(message, type) {
        // Update status message
        this.statusMessage.textContent = message;
        this.statusMessage.className = 'status-message';
        this.statusMessage.classList.add(type);
        this.statusMessage.classList.remove('hidden');

        // Auto-hide after 5 seconds
        if (this.statusTimeout) {
            clearTimeout(this.statusTimeout);
        }
        
        this.statusTimeout = setTimeout(() => {
            this.statusMessage.classList.add('hidden');
        }, 5000);
    }
}

// Initialize app when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    new ProDnBApp();
});

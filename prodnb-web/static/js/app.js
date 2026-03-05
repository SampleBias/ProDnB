// ProDnB Web UI - Main Application Logic

class ProDnBApp {
    constructor() {
        this.uploadedFilePath = null;
        this.proteinInfo = null;
        this.mappedPrimitives = null;  // { tempo, primitives, rhythm_seed, chain_lengths, element_counts }

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
        this.generateStreamBtn = document.getElementById('generateStreamBtn');
        this.btnText = this.generateBtn.querySelector('.btn-text');
        this.btnLoading = this.generateBtn.querySelector('.btn-loading');
        this.streamBtnText = this.generateStreamBtn?.querySelector('.btn-text');
        this.streamBtnLoading = this.generateStreamBtn?.querySelector('.btn-loading');

        // Sliders
        this.slidersSection = document.getElementById('slidersSection');
        this.kickSlider = document.getElementById('kickSlider');
        this.snareSlider = document.getElementById('snareSlider');
        this.hatsSlider = document.getElementById('hatsSlider');
        this.energySlider = document.getElementById('energySlider');
        this.kickValue = document.getElementById('kickValue');
        this.snareValue = document.getElementById('snareValue');
        this.hatsValue = document.getElementById('hatsValue');
        this.energyValue = document.getElementById('energyValue');

        // Piano roll
        this.pianoRollSection = document.getElementById('pianoRollSection');
        this.pianoRollGrid = document.getElementById('pianoRollGrid');

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

        // Generate (Stream)
        if (this.generateStreamBtn) {
            this.generateStreamBtn.addEventListener('click', () => {
                this.generateStrudelStream();
            });
        }

        // Sliders
        if (this.kickSlider) {
            this.kickSlider.addEventListener('input', () => this.onSliderChange());
            this.kickSlider.addEventListener('change', () => this.onSliderChange());
        }
        if (this.snareSlider) {
            this.snareSlider.addEventListener('input', () => this.onSliderChange());
            this.snareSlider.addEventListener('change', () => this.onSliderChange());
        }
        if (this.hatsSlider) {
            this.hatsSlider.addEventListener('input', () => this.onSliderChange());
            this.hatsSlider.addEventListener('change', () => this.onSliderChange());
        }
        if (this.energySlider) {
            this.energySlider.addEventListener('input', () => this.onSliderChange());
            this.energySlider.addEventListener('change', () => this.onSliderChange());
        }

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
        const validExtensions = ['.pdb', '.ent', '.cif'];
        const fileName = file.name.toLowerCase();
        const isValid = validExtensions.some(ext => fileName.endsWith(ext));

        if (!isValid) {
            this.showStatus('Please upload a PDB, ENT, or CIF file', 'error');
            return;
        }

        if (file.size > 10 * 1024 * 1024) {
            this.showStatus('File too large. Maximum size is 10MB', 'error');
            return;
        }

        this.fileName.textContent = `${file.name} (uploading...)`;
        this.fileInfo.classList.remove('hidden');
        this.uploadArea.classList.add('hidden');

        try {
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

            this.uploadedFilePath = result.file_path;
            this.proteinInfo = {
                chain_count: result.chain_count,
                residue_count: result.residue_count,
                atom_count: result.atom_count
            };

            this.fileName.textContent = file.name;
            this.fileStats.textContent = `Chains: ${result.chain_count} | Residues: ${result.residue_count} | Atoms: ${result.atom_count}`;
            this.generateBtn.disabled = false;
            if (this.generateStreamBtn) this.generateStreamBtn.disabled = false;

            this.showStatus('PDB file uploaded successfully!', 'success');

            // Map to primitives (stage 1, deterministic)
            await this.fetchMap();
        } catch (error) {
            console.error('Upload error:', error);
            this.clearFileUpload();
            this.showStatus(error.message || 'Failed to upload file', 'error');
        }
    }

    async fetchMap() {
        if (!this.uploadedFilePath) return;

        try {
            const response = await fetch('/api/map', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ file_path: this.uploadedFilePath, bpm: 174 })
            });

            if (!response.ok) {
                const err = await response.json().catch(() => ({}));
                const msg = err.error || err.message || `Map failed (${response.status})`;
                throw new Error(msg);
            }

            this.mappedPrimitives = await response.json();

            // Show sliders and piano roll
            if (this.slidersSection) {
                this.slidersSection.classList.remove('hidden');
                this.updateSliderValuesFromPrimitives();
            }
            if (this.pianoRollSection) {
                this.pianoRollSection.classList.remove('hidden');
                this.renderPianoRoll();
            }

            // Assemble from primitives (no LLM) for initial display
            await this.assembleFromSliders();
        } catch (error) {
            console.error('Map error:', error);
            this.showStatus(error.message || 'Failed to map primitives', 'error');
        }
    }

    updateSliderValuesFromPrimitives() {
        if (!this.mappedPrimitives?.primitives) return;

        for (const p of this.mappedPrimitives.primitives) {
            const gainPct = Math.round((p.gain || 0.8) * 100);
            if (p.layer === 'kick' && this.kickSlider) {
                this.kickSlider.value = gainPct;
                if (this.kickValue) this.kickValue.textContent = gainPct + '%';
            } else if (p.layer === 'snare' && this.snareSlider) {
                this.snareSlider.value = gainPct;
                if (this.snareValue) this.snareValue.textContent = gainPct + '%';
            } else if (p.layer === 'hats' && this.hatsSlider) {
                this.hatsSlider.value = gainPct;
                if (this.hatsValue) this.hatsValue.textContent = gainPct + '%';
            }
        }
    }

    renderPianoRoll() {
        if (!this.pianoRollGrid || !this.mappedPrimitives?.primitives) return;

        const primitives = this.mappedPrimitives.primitives;
        const cols = 16;

        // Parse pattern to events per row (simplified: expand pattern to 16 slots)
        const expandPattern = (pattern) => {
            if (!pattern) return Array(cols).fill('~');
            const parts = pattern.split(/\s+/);
            const out = [];
            for (let i = 0; i < cols; i++) {
                const idx = i % parts.length;
                out.push(parts[idx] === '~' ? '' : parts[idx]);
            }
            return out;
        };

        let html = '';

        // Header row
        html += '<div class="piano-roll-cell header"></div>';
        for (let c = 0; c < cols; c++) {
            html += `<div class="piano-roll-cell header">${c + 1}</div>`;
        }

        for (const p of primitives) {
            const layer = p.layer || p.primitive_type;
            let events;
            if (p.primitive_type === 'euclidean' && p.sound && p.beats != null && p.segments != null) {
                events = this.euclideanToEvents(p.sound, p.beats, p.segments, cols);
            } else {
                events = expandPattern(p.pattern);
            }

            html += `<div class="piano-roll-cell header">${layer}</div>`;
            for (const ev of events) {
                const cls = ev ? 'active' : 'rest';
                html += `<div class="piano-roll-cell ${cls}">${ev || ''}</div>`;
            }
        }

        this.pianoRollGrid.innerHTML = html;
    }

    euclideanToEvents(sound, beats, segments, cols) {
        const out = Array(cols).fill('');
        for (let i = 0; i < beats; i++) {
            const pos = Math.floor((i * segments) / beats) % cols;
            out[pos] = sound;
        }
        return out;
    }

    getSliderValues() {
        return {
            kick: this.kickSlider ? parseInt(this.kickSlider.value, 10) / 100 : undefined,
            snare: this.snareSlider ? parseInt(this.snareSlider.value, 10) / 100 : undefined,
            hats: this.hatsSlider ? parseInt(this.hatsSlider.value, 10) / 100 : undefined,
            energy: this.energySlider ? parseInt(this.energySlider.value, 10) / 100 : undefined
        };
    }

    async onSliderChange() {
        if (this.kickValue) this.kickValue.textContent = (this.kickSlider?.value || 90) + '%';
        if (this.snareValue) this.snareValue.textContent = (this.snareSlider?.value || 85) + '%';
        if (this.hatsValue) this.hatsValue.textContent = (this.hatsSlider?.value || 60) + '%';
        if (this.energyValue) this.energyValue.textContent = (this.energySlider?.value || 100) + '%';

        await this.assembleFromSliders();
    }

    async assembleFromSliders() {
        if (!this.mappedPrimitives) return;

        try {
            const sliders = this.getSliderValues();
            const response = await fetch('/api/assemble', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    primitives: this.mappedPrimitives.primitives,
                    tempo: this.mappedPrimitives.tempo || 174,
                    sliders: sliders
                })
            });

            if (!response.ok) {
                throw new Error('Assemble failed');
            }

            const result = await response.json();
            if (result.success && result.code) {
                this.strudelCode.textContent = result.code;
                this.strudelCode.style.color = '';
                this.copyBtn.disabled = false;
                this.clearBtn.classList.remove('hidden');
            }
        } catch (error) {
            console.error('Assemble error:', error);
        }
    }

    clearFileUpload() {
        this.uploadedFilePath = null;
        this.proteinInfo = null;
        this.mappedPrimitives = null;
        this.fileInput.value = '';
        this.fileInfo.classList.add('hidden');
        this.uploadArea.classList.remove('hidden');
        this.generateBtn.disabled = true;
        if (this.generateStreamBtn) this.generateStreamBtn.disabled = true;
        if (this.slidersSection) this.slidersSection.classList.add('hidden');
        if (this.pianoRollSection) this.pianoRollSection.classList.add('hidden');
    }

    async generateStrudel() {
        if (!this.uploadedFilePath) {
            this.showStatus('Please upload a PDB file first', 'error');
            return;
        }

        this.setGeneratingState(true, false);
        this.strudelCode.textContent = '// Generating Strudel code...';
        this.strudelCode.style.color = '#666666';

        try {
            const response = await fetch('/api/generate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ file_path: this.uploadedFilePath })
            });

            const result = await response.json();

            if (!response.ok || !result.success) {
                throw new Error(result.error || 'Generation failed');
            }

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
            this.setGeneratingState(false, false);
        }
    }

    async generateStrudelStream() {
        if (!this.uploadedFilePath) {
            this.showStatus('Please upload a PDB file first', 'error');
            return;
        }

        this.setGeneratingState(false, true);
        this.strudelCode.textContent = '// Streaming...';
        this.strudelCode.style.color = '#666666';

        try {
            const response = await fetch('/api/generate/stream', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ file_path: this.uploadedFilePath })
            });

            if (!response.ok) {
                const err = await response.json().catch(() => ({}));
                throw new Error(err.error || 'Stream failed');
            }

            const reader = response.body.getReader();
            const decoder = new TextDecoder();
            let code = '';
            let buffer = '';

            while (true) {
                const { done, value } = await reader.read();
                if (done) break;

                buffer += decoder.decode(value, { stream: true });
                const parts = buffer.split('\n');
                buffer = parts.pop() || '';

                for (const line of parts) {
                    if (line.startsWith('data: ')) {
                        const data = line.slice(6).trim();
                        if (data === '[DONE]') continue;

                        try {
                            const payload = JSON.parse(data);

                            if (payload.chunk_type === 'chunk' && payload.content) {
                                code += payload.content;
                                this.strudelCode.textContent = code;
                            } else if (payload.chunk_type === 'error') {
                                throw new Error(payload.content || 'Stream error');
                            }
                        } catch (e) {
                            if (e instanceof SyntaxError) continue;
                            throw e;
                        }
                    }
                }
            }

            // Strip markdown code blocks if present
            let finalCode = code.trim();
            if (finalCode.startsWith('```')) {
                finalCode = finalCode.replace(/^```(?:javascript|js|strudel)?\n?/, '').replace(/\n?```$/, '').trim();
            }

            this.strudelCode.textContent = finalCode || code;
            this.strudelCode.style.color = '';
            this.copyBtn.disabled = false;
            this.clearBtn.classList.remove('hidden');

            this.showStatus('Strudel code streamed successfully!', 'success');
        } catch (error) {
            console.error('Stream error:', error);
            this.strudelCode.textContent = `// Error: ${error.message}`;
            this.strudelCode.style.color = '#b91c1c';
            this.showStatus(error.message || 'Failed to stream', 'error');
        } finally {
            this.setGeneratingState(false, true);
        }
    }

    setGeneratingState(isGenerating, isStream) {
        if (isStream) {
            this.generateStreamBtn.disabled = isGenerating;
            if (isGenerating) {
                this.streamBtnText.classList.add('hidden');
                this.streamBtnLoading.classList.remove('hidden');
            } else {
                this.streamBtnText.classList.remove('hidden');
                this.streamBtnLoading.classList.add('hidden');
            }
        } else {
            this.generateBtn.disabled = isGenerating;
            if (isGenerating) {
                this.btnText.classList.add('hidden');
                this.btnLoading.classList.remove('hidden');
            } else {
                this.btnText.classList.remove('hidden');
                this.btnLoading.classList.add('hidden');
            }
        }
    }

    copyCode() {
        const code = this.strudelCode.textContent;

        navigator.clipboard.writeText(code).then(() => {
            this.showStatus('Code copied to clipboard!', 'success');

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
        this.statusMessage.textContent = message;
        this.statusMessage.className = 'status-message';
        this.statusMessage.classList.add(type);
        this.statusMessage.classList.remove('hidden');

        if (this.statusTimeout) {
            clearTimeout(this.statusTimeout);
        }

        this.statusTimeout = setTimeout(() => {
            this.statusMessage.classList.add('hidden');
        }, 5000);
    }
}

document.addEventListener('DOMContentLoaded', () => {
    new ProDnBApp();
});

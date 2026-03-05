// ProDnB Web UI - Main Application Logic

class ProDnBApp {
    constructor() {
        this.uploadedFilePath = null;
        this.proteinInfo = null;
        this.mappedPrimitives = null;  // { tempo, primitives, rhythm_seed, chain_lengths, element_counts }
        this.proteinFunctions = [];    // [{ title, snippet }]
        this.selectedFunction = null;  // selected snippet text for infer-beat-design
        this.orchestrationInstruction = null;  // user-editable orchestration prompt for code gen

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
        this.btnText = this.generateBtn?.querySelector('.btn-text');
        this.btnLoading = this.generateBtn?.querySelector('.btn-loading');

        // Genre elements
        this.genreSelect = document.getElementById('genreSelect');
        this.keySelect = document.getElementById('keySelect');
        this.octaveInput = document.getElementById('octaveInput');
        this.melodicCheck = document.getElementById('melodicCheck');

        // Output elements
        this.strudelCode = document.getElementById('strudelCode');
        this.copyBtn = document.getElementById('copyBtn');
        this.clearBtn = document.getElementById('clearBtn');

        // Function section (Step 2)
        this.functionSection = document.getElementById('functionSection');
        this.functionHint = document.getElementById('functionHint');
        this.findFunctionBtn = document.getElementById('findFunctionBtn');
        this.functionLoading = document.getElementById('functionLoading');
        this.functionList = document.getElementById('functionList');
        this.clearFunctionBtn = document.getElementById('clearFunction');
        this.continueSection = document.getElementById('continueSection');
        this.continueJourneyBtn = document.getElementById('continueJourneyBtn');
        this.orchestrationLoading = document.getElementById('orchestrationLoading');
        this.orchestrationEditor = document.getElementById('orchestrationEditor');
        this.orchestrationInstructionEl = document.getElementById('orchestrationInstruction');
        this.genreSection = document.getElementById('genreSection');

        // Status message
        this.statusMessage = document.getElementById('statusMessage');
    }

    getGenreParams() {
        const genre = this.genreSelect?.value || '';
        const key = this.keySelect?.value || '';
        const octave = this.octaveInput?.value ? parseInt(this.octaveInput.value, 10) : null;
        const melodic = this.melodicCheck?.checked || false;
        const orchestration = (this.orchestrationInstructionEl?.value || this.orchestrationInstruction || '').trim();
        this.orchestrationInstruction = orchestration || null;

        const params = {
            ...(genre && { genre }),
            ...(key && { key }),
            ...(octave >= 2 && octave <= 5 && { octave }),
            ...(melodic && { melodic })
        };
        if (this.selectedFunction) {
            params.selected_function = this.selectedFunction;
        }
        if (this.orchestrationInstruction) {
            params.orchestration_instruction = this.orchestrationInstruction;
        }
        return params;
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

        // Genre options change → re-map and reassemble
        [this.genreSelect, this.keySelect, this.octaveInput, this.melodicCheck].forEach(el => {
            if (el) {
                el.addEventListener('change', () => {
                    if (this.uploadedFilePath) this.fetchMap();
                });
            }
        });
        // Generate Strudel code (streams output)
        if (this.generateBtn) {
            this.generateBtn.addEventListener('click', () => this.generateStrudel());
        }

        // Copy code
        this.copyBtn.addEventListener('click', () => {
            this.copyCode();
        });

        // Clear code
        this.clearBtn.addEventListener('click', () => {
            this.clearOutput();
        });

        // Find function button – SERPAPI search
        if (this.findFunctionBtn) {
            this.findFunctionBtn.addEventListener('click', () => this.handleFindFunction());
        }

        // Continue the journey – generate orchestration instruction
        if (this.continueJourneyBtn) {
            this.continueJourneyBtn.addEventListener('click', () => this.handleContinueJourney());
        }

        // Sync orchestration textarea to state before Generate
        if (this.orchestrationInstructionEl) {
            this.orchestrationInstructionEl.addEventListener('input', () => {
                this.orchestrationInstruction = this.orchestrationInstructionEl.value.trim() || null;
            });
        }

        // Clear function selection
        if (this.clearFunctionBtn) {
            this.clearFunctionBtn.addEventListener('click', () => {
                this.selectedFunction = null;
                this.orchestrationInstruction = null;
                if (this.orchestrationInstructionEl) this.orchestrationInstructionEl.value = '';
                if (this.continueSection) this.continueSection.classList.add('hidden');
                if (this.orchestrationEditor) this.orchestrationEditor.classList.add('hidden');
                this.renderFunctionList();
                if (this.uploadedFilePath) this.fetchMap();
            });
        }
    }

    async handleFindFunction() {
        if (!this.uploadedFilePath) return;

        this.setStep2Loading(true);
        this.setStep3Enabled(false);
        this.findFunctionBtn.disabled = true;

        try {
            await this.fetchProteinFunction();
            this.setStep3Enabled(true);
        } catch (e) {
            this.showStatus('Failed to fetch protein function', 'error');
        } finally {
            this.setStep2Loading(false);
            this.findFunctionBtn.disabled = false;
        }
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

            this.showStatus('PDB file uploaded successfully!', 'success');

            this.findFunctionBtn.disabled = false;
            this.setStep3Enabled(false);

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
            const genreParams = this.getGenreParams();
            const response = await fetch('/api/map', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ file_path: this.uploadedFilePath, bpm: 174, ...genreParams })
            });

            if (!response.ok) {
                const err = await response.json().catch(() => ({}));
                let msg = err.error || err.message || `Map failed (${response.status})`;
                if (response.status === 404) {
                    msg = 'API not found (404). Rebuild and restart the server: cargo build -p prodnb-web && cargo run -p prodnb-web';
                }
                throw new Error(msg);
            }

            this.mappedPrimitives = await response.json();

            // Assemble from primitives (no LLM) for initial display - intensity & piano roll in Strudel output
            await this.assembleFromPrimitives();
        } catch (error) {
            console.error('Map error:', error);
            this.showStatus(error.message || 'Failed to map primitives', 'error');
            // Don't block - user can still use Generate
        }
    }

    async assembleFromPrimitives() {
        if (!this.mappedPrimitives) return;

        try {
            const genreParams = this.getGenreParams();
            const response = await fetch('/api/assemble', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    primitives: this.mappedPrimitives.primitives,
                    tempo: this.mappedPrimitives.tempo || 174,
                    ...genreParams
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

    setStep2Loading(loading) {
        if (this.functionHint) {
            this.functionHint.classList.toggle('hidden', loading || !!this.proteinFunctions?.length);
        }
        if (this.findFunctionBtn) {
            this.findFunctionBtn.classList.toggle('hidden', loading);
        }
        if (this.functionLoading) {
            this.functionLoading.classList.toggle('hidden', !loading);
            this.functionLoading.textContent = loading ? 'Searching for protein function...' : '';
        }
        if (this.clearFunctionBtn) {
            this.clearFunctionBtn.classList.toggle('hidden', loading);
        }
    }

    setStep3Enabled(enabled) {
        this.generateBtn.disabled = !enabled;
        if (this.genreSection) {
            this.genreSection.classList.toggle('step-disabled', !enabled);
        }
    }

    async fetchProteinFunction() {
        if (!this.uploadedFilePath) return;

        if (this.functionHint) this.functionHint.classList.add('hidden');
        if (this.functionList) this.functionList.innerHTML = '';
        this.proteinFunctions = [];
        this.selectedFunction = null;

        try {
            const response = await fetch('/api/protein-function', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ file_path: this.uploadedFilePath })
            });

            if (!response.ok) {
                throw new Error('Failed to fetch protein function');
            }

            const result = await response.json();
            this.proteinFunctions = result.functions || [];
            this.renderFunctionList();
        } catch (error) {
            console.warn('Protein function fetch failed:', error);
            this.proteinFunctions = [];
            this.renderFunctionList();
            this.showStatus('Protein function lookup failed. Use genre options below.', 'error');
        } finally {
            // Step 2 loading cleared by handleFileUpload after await
        }
    }

    renderFunctionList() {
        if (!this.functionList) return;

        this.functionList.innerHTML = '';
        for (const fn of this.proteinFunctions) {
            const text = [fn.title, fn.snippet].filter(Boolean).join(' — ');
            if (!text) continue;

            const card = document.createElement('button');
            card.type = 'button';
            card.className = 'function-card' + (this.selectedFunction === text ? ' selected' : '');
            card.innerHTML = `
                <div class="function-card-title">${this.escapeHtml(fn.title || 'Function')}</div>
                <div class="function-card-snippet">${this.escapeHtml(fn.snippet || '')}</div>
            `;
            card.addEventListener('click', () => {
                this.selectedFunction = text;
                this.renderFunctionList();
                this.showContinueSection();
                this.fetchInferredRecommendations();
                if (this.uploadedFilePath) this.fetchMap();
            });
            this.functionList.appendChild(card);
        }

        if (this.proteinFunctions.length === 0 && this.uploadedFilePath) {
            this.functionList.innerHTML = '<p class="hint">No function descriptions found. Use genre options below.</p>';
        }

        if (this.clearFunctionBtn && this.uploadedFilePath) {
            this.clearFunctionBtn.classList.remove('hidden');
        }
    }

    showContinueSection() {
        if (this.continueSection && this.selectedFunction) {
            this.continueSection.classList.remove('hidden');
            this.orchestrationInstruction = null;
            if (this.orchestrationInstructionEl) this.orchestrationInstructionEl.value = '';
            if (this.orchestrationEditor) this.orchestrationEditor.classList.add('hidden');
        }
    }

    async handleContinueJourney() {
        if (!this.selectedFunction) return;

        if (this.orchestrationLoading) this.orchestrationLoading.classList.remove('hidden');
        if (this.continueJourneyBtn) this.continueJourneyBtn.disabled = true;

        try {
            const genre = this.genreSelect?.value || '';
            const key = this.keySelect?.value || '';
            const melodic = this.melodicCheck?.checked || false;

            const response = await fetch('/api/generate-orchestration-instruction', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    selected_function: this.selectedFunction,
                    ...(genre && { genre }),
                    ...(key && { key }),
                    melodic
                })
            });

            if (!response.ok) {
                const err = await response.json().catch(() => ({}));
                throw new Error(err.error || 'Failed to generate instruction');
            }

            const result = await response.json();
            const instruction = result.instruction || '';
            this.orchestrationInstruction = instruction;
            if (this.orchestrationInstructionEl) {
                this.orchestrationInstructionEl.value = instruction;
            }
            if (this.orchestrationEditor) this.orchestrationEditor.classList.remove('hidden');
            this.showStatus('Orchestration instruction ready. Edit if needed, then Generate.', 'success');
        } catch (error) {
            console.error('Continue journey error:', error);
            this.showStatus(error.message || 'Failed to generate orchestration instruction', 'error');
        } finally {
            if (this.orchestrationLoading) this.orchestrationLoading.classList.add('hidden');
            if (this.continueJourneyBtn) this.continueJourneyBtn.disabled = false;
        }
    }

    async fetchInferredRecommendations() {
        if (!this.selectedFunction) return;

        try {
            const response = await fetch('/api/infer-beat-design', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ selected_function: this.selectedFunction })
            });

            if (!response.ok) {
                const err = await response.json().catch(() => ({}));
                throw new Error(err.error || 'Inference failed');
            }

            const inferred = await response.json();
            if (this.genreSelect && inferred.genre) {
                this.genreSelect.value = inferred.genre;
            }
            if (this.keySelect && inferred.key) {
                const key = this.normalizeKeyForDropdown(inferred.key);
                if (this.keySelect.querySelector(`option[value="${key}"]`)) {
                    this.keySelect.value = key;
                }
            }
            if (this.melodicCheck) {
                this.melodicCheck.checked = !!inferred.melodic;
            }
            if (this.octaveInput) {
                const oct = inferred.octave != null ? Math.min(5, Math.max(2, inferred.octave)) : 3;
                this.octaveInput.value = String(oct);
            }
            this.showStatus(`Recommended: ${inferred.genre || 'default'} @ ${inferred.bpm || 174} BPM`, 'success');
        } catch (error) {
            console.warn('Could not fetch recommendations:', error);
        }
    }

    /** Normalize API key (e.g. "C:minor", "g:minor") to dropdown value ("Cm", "Gm") */
    normalizeKeyForDropdown(key) {
        if (!key || typeof key !== 'string') return '';
        const k = key.trim().toLowerCase();
        const minorMap = { 'c': 'Cm', 'd': 'Dm', 'e': 'Em', 'f': 'Fm', 'g': 'Gm', 'a': 'Am', 'b': 'Bm' };
        const minorMatch = k.match(/^([a-g])(?:#|b)?\s*:\s*minor$/);
        if (minorMatch) return minorMap[minorMatch[1]] || (minorMatch[1].toUpperCase() + 'm');
        if (['am', 'bm', 'cm', 'dm', 'em', 'fm', 'gm'].includes(k)) return k.charAt(0).toUpperCase() + 'm';
        return key;
    }

    escapeHtml(s) {
        const div = document.createElement('div');
        div.textContent = s;
        return div.innerHTML;
    }

    clearFileUpload() {
        this.uploadedFilePath = null;
        this.proteinInfo = null;
        this.mappedPrimitives = null;
        this.proteinFunctions = [];
        this.selectedFunction = null;
        this.fileInput.value = '';
        this.fileInfo.classList.add('hidden');
        this.uploadArea.classList.remove('hidden');
        this.generateBtn.disabled = true;
        if (this.functionHint) {
            this.functionHint.textContent = 'Upload a PDB file above, then click below to search for the protein\'s biological function via Google (SERPAPI).';
            this.functionHint.classList.remove('hidden');
        }
        if (this.findFunctionBtn) {
            this.findFunctionBtn.disabled = true;
            this.findFunctionBtn.classList.remove('hidden');
        }
        if (this.functionLoading) this.functionLoading.classList.add('hidden');
        if (this.functionList) this.functionList.innerHTML = '';
        if (this.clearFunctionBtn) this.clearFunctionBtn.classList.add('hidden');
        if (this.continueSection) this.continueSection.classList.add('hidden');
        if (this.orchestrationInstructionEl) this.orchestrationInstructionEl.value = '';
        this.orchestrationInstruction = null;
        if (this.genreSection) this.genreSection.classList.remove('step-disabled');
    }

    async generateStrudel() {
        if (!this.uploadedFilePath) {
            this.showStatus('Please upload a PDB file first', 'error');
            return;
        }

        this.setGeneratingState(true);
        this.strudelCode.textContent = '// Generating...';
        this.strudelCode.style.color = '#666666';

        try {
            const genreParams = this.getGenreParams();
            const response = await fetch('/api/generate/stream', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ file_path: this.uploadedFilePath, ...genreParams })
            });

            if (!response.ok) {
                const err = await response.json().catch(() => ({}));
                throw new Error(err.error || 'Stream failed');
            }

            const reader = response.body.getReader();
            const decoder = new TextDecoder();
            let code = '';
            let headerChunk = '';
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

                            if (payload.chunk_type === 'header' && payload.content) {
                                headerChunk = payload.content;
                            } else if (payload.chunk_type === 'chunk' && payload.content) {
                                code += payload.content;
                                this.strudelCode.textContent = (headerChunk + code);
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
            finalCode = this.fixEuclideanOrder(finalCode);
            finalCode = this.fixTidalToJs(finalCode);
            finalCode = this.ensureAcidenvRegistered(finalCode);
            finalCode = this.ensureStackOutput(finalCode);
            finalCode = this.synthFallback(finalCode);
            if (headerChunk && !finalCode.includes('REPRESENTATION KEY')) {
                finalCode = headerChunk + finalCode;
            }

            this.strudelCode.textContent = finalCode || code;
            this.strudelCode.style.color = '';
            this.copyBtn.disabled = false;
            this.clearBtn.classList.remove('hidden');

            this.showStatus('Strudel code generated successfully!', 'success');
        } catch (error) {
            console.error('Stream error:', error);
            this.strudelCode.textContent = `// Error: ${error.message}`;
            this.strudelCode.style.color = '#b91c1c';
            this.showStatus(error.message || 'Failed to generate code', 'error');
        } finally {
            this.setGeneratingState(false);
        }
    }

    setGeneratingState(isGenerating) {
        if (!this.generateBtn) return;
        this.generateBtn.disabled = isGenerating;
        if (isGenerating) {
            this.btnText?.classList.add('hidden');
            this.btnLoading?.classList.remove('hidden');
        } else {
            this.btnText?.classList.remove('hidden');
            this.btnLoading?.classList.add('hidden');
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

    /** Fix reversed euclidean: (5,8)bd -> bd(5,8) */
    fixEuclideanOrder(code) {
        return code.replace(/"\((\d+),(\d+)\)(bd|sd|hh|cp|rim|oh|perc)"/g, '"$3($1,$2)"');
    }

    /** Convert Tidal syntax to Strudel JS: remove d1 $, stack([...]) -> stack(...) */
    fixTidalToJs(code) {
        let out = code.replace(/\bd\d+\s*\$?\s*/g, '');
        out = out.replace(/stack\(\s*\[/g, 'stack(');
        // Replace only the last ]) to avoid breaking nested stacks
        const last = out.lastIndexOf('])');
        if (last >= 0) out = out.slice(0, last) + ')' + out.slice(last + 2);
        return out;
    }

    /** Inject register('acidenv', ...) after setcps if .acidenv used but not registered */
    ensureAcidenvRegistered(code) {
        if (!code.includes('.acidenv')) return code;
        if (/register\s*\(\s*['"]acidenv['"]/.test(code)) return code;
        const registerBlock = `register('acidenv', (x, pat) => pat
  .lpf(100)
  .lpenv(x * 9)
  .lps(0.2)
  .lpd(0.12)
);
`;
        const setcpsMatch = code.match(/setcps\s*\([^)]+\)\s*;?\s*/);
        if (setcpsMatch) {
            const idx = code.indexOf(setcpsMatch[0]) + setcpsMatch[0].length;
            return code.slice(0, idx) + '\n' + registerBlock + code.slice(idx);
        }
        return `setcps(174/60/4);\n${registerBlock}${code}`;
    }

    /** Synth fallback: sine/sawtooth can be silent; triangle is most reliable */
    synthFallback(code) {
        return code.replace(/\.s\("sine"\)/g, '.s("triangle")').replace(/\.s\('sine'\)/g, ".s('triangle')");
    }

    /** Ensure final stack(drums, bass, pad, lead) so output actually plays */
    ensureStackOutput(code) {
        const trimmed = code.trim();
        const layers = [];
        for (const name of ['drums', 'bass', 'pad', 'lead']) {
            if (new RegExp(`const\\s+${name}\\s*=`).test(code)) layers.push(name);
        }
        if (layers.length === 0) return code;
        const stackCall = `stack(${layers.join(', ')})`;
        if (trimmed.endsWith(stackCall) || trimmed.endsWith(stackCall + '\n')) return code;
        return code + (code.endsWith('\n') ? '' : '\n') + '\n' + stackCall;
    }
}

document.addEventListener('DOMContentLoaded', () => {
    new ProDnBApp();
});

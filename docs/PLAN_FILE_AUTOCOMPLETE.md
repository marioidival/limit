# Plano de Implementação: Autocomplete de Arquivos com @

**Data:** 2025-03-11  
**Objetivo:** Implementar autocomplete de arquivos quando o usuário digita `@` no input, similar ao GitHub Copilot/Cursor

## Visão Geral

Quando o usuário digita `@` no campo de input do TUI, o sistema deve:
1. Detectar o caractere `@` como trigger de autocomplete
2. Mostrar uma lista de arquivos do diretório atual (ou do diretório de trabalho)
3. Filtrar arquivos baseado no texto digitado após `@` (ex: `@Cargo` mostra `Cargo.toml`, `Cargo.lock`)
4. Permitir navegação com setas e seleção com Enter/Tab
5. Inserir o caminho do arquivo selecionado no input

## Análise do Código Atual

### Componentes Envolvidos

1. **`TuiApp`** (`limit-cli/src/tui_bridge.rs`)
   - Gerencia estado do input: `input_text: String`, `cursor_pos: usize`
   - Processa eventos de teclado em `handle_key_event()`
   - Renderiza UI em `draw_ui()`

2. **`InputPrompt`** (`limit-tui/src/components/prompt.rs`)
   - Componente reutilizável de input
   - Atualmente não usado no TUI principal (TuiApp tem sua própria implementação)

### Estado Necessário

```rust
pub struct FileAutocompleteState {
    /// Whether autocomplete popup is visible
    is_active: bool,
    /// Query typed after @ (e.g., "Cargo" in "@Cargo")
    query: String,
    /// Start position of @ in input_text
    trigger_pos: usize,
    /// List of matching files
    matches: Vec<FileMatch>,
    /// Currently selected index in matches
    selected_index: usize,
    /// Working directory for file search
    working_dir: PathBuf,
}

pub struct FileMatch {
    /// Relative path from working_dir
    path: PathBuf,
    /// Whether it's a directory
    is_dir: bool,
    /// Fuzzy match score
    score: i64,
}
```

## Bibliotecas Recomendadas

### 1. **Frizbee** (RECOMENDADO - Mais moderno e rápido)
- **Versão:** 0.8.3 (Março 2026)
- **URL:** https://lib.rs/crates/frizbee
- **Por que:** 
  - SIMD-optimized fuzzy matching (1.7x mais rápido que nucleo, 2.1x que fzf)
  - Usado por blink.cmp, skim, fff.nvim
  - API simples: `match_list(needle, &haystacks, Config::default())`
  - Suporte a typo-resistance
- **Cargo.toml:**
  ```toml
  frizbee = "0.8"
  ```

### 2. **Nucleo** (ALTERNATIVA - Do Helix Editor)
- **Versão:** 0.3+
- **URL:** https://github.com/helix-editor/nucleo
- **Por que:**
  - Desenvolvido para o Helix editor
  - API de alto nível com `nucleo::Matcher`
  - Bem testado e estável
- **Cargo.toml:**
  ```tomv
  nucleo = "0.3"
  nucleo-matcher = "0.3"
  ```

### 3. **fuzzy-matcher** (ALTERNATIVA CLÁSSICA)
- **Versão:** 0.3.7
- **URL:** https://github.com/skim-rs/fuzzy-matcher
- **Por que:**
  - Usado pelo skim (fuzzy finder em Rust)
  - API simples com `SkimMatcherV2`
  - Bem estabelecido
- **Cargo.toml:**
  ```toml
  fuzzy-matcher = "0.3"
  ```

### 4. **Biblioteca de File System**
Já temos `walkdir` indiretamente? Não, precisamos adicionar:
```toml
walkdir = "2.5"  # Para escanear diretórios de forma eficiente
glob = "0.3"      # Para padrões de ignore como .gitignore
```

## Arquitetura da Solução

### Componentes Novos

```
limit-tui/src/components/
├── file_autocomplete.rs  (NOVO - Widget de autocomplete)
└── mod.rs                (Atualizar exports)

limit-cli/src/
├── file_finder.rs        (NOVO - Lógica de busca de arquivos)
└── tui_bridge.rs         (Atualizar TuiApp)
```

### Fluxo de Dados

```
Usuário digita @
    ↓
TuiApp detecta '@' em handle_key_event()
    ↓
Ativa FileAutocompleteState
    ↓
FileFinder::scan_directory() → Lista de arquivos
    ↓
Frizbee::match_list() → Filtra por query
    ↓
Renderiza popup com FileAutocompleteWidget
    ↓
Usuário navega (↑/↓) e seleciona (Enter/Tab)
    ↓
Insere caminho no input_text
```

## Plano de Implementação Detalhado

### FASE 1: Infraestrutura (Backend)

#### 1.1. Criar `FileFinder` (`limit-cli/src/file_finder.rs`)

```rust
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileFinder {
    working_dir: PathBuf,
    ignore_patterns: Vec<glob::Pattern>,
}

impl FileFinder {
    pub fn new(working_dir: PathBuf) -> Self {
        // Carregar .gitignore, .ignore, etc.
    }
    
    /// Scan directory and return all files (relative paths)
    pub fn scan_files(&self, max_depth: usize) -> Vec<PathBuf> {
        // Usar WalkDir com ignore patterns
    }
    
    /// Filter files by query using fuzzy matching
    pub fn filter_files(&self, files: &[PathBuf], query: &str) -> Vec<FileMatch> {
        // Usar frizbee::match_list()
    }
}
```

#### 1.2. Adicionar dependências

```toml
# limit-cli/Cargo.toml
[dependencies]
frizbee = "0.8"
walkdir = "2.5"
glob = "0.3"
```

### FASE 2: Estado no TuiApp

#### 2.1. Adicionar estado em `TuiApp`

```rust
pub struct TuiApp {
    // ... existing fields ...
    
    /// File autocomplete state
    file_autocomplete: Option<FileAutocompleteState>,
    /// File finder instance
    file_finder: FileFinder,
    /// Cached file list (refreshed periodically)
    cached_files: Vec<PathBuf>,
}
```

#### 2.2. Detectar `@` em `handle_key_event()`

```rust
KeyCode::Char('@') => {
    // Insert @ character
    self.input_text.insert(self.cursor_pos, '@');
    self.cursor_pos += 1;
    
    // Activate autocomplete
    self.file_autocomplete = Some(FileAutocompleteState {
        is_active: true,
        query: String::new(),
        trigger_pos: self.cursor_pos - 1,
        matches: self.get_file_matches(""),
        selected_index: 0,
        working_dir: self.file_finder.working_dir.clone(),
    });
}

KeyCode::Char(c) if self.file_autocomplete.is_some() => {
    // Add to query and filter
    if let Some(ref mut ac) = self.file_autocomplete {
        ac.query.push(c);
        ac.matches = self.get_file_matches(&ac.query);
        ac.selected_index = 0;
    }
    self.input_text.insert(self.cursor_pos, c);
    self.cursor_pos += 1;
}

KeyCode::Up if self.file_autocomplete.is_some() => {
    if let Some(ref mut ac) = self.file_autocomplete {
        if ac.selected_index > 0 {
            ac.selected_index -= 1;
        }
    }
}

KeyCode::Down if self.file_autocomplete.is_some() => {
    if let Some(ref mut ac) = self.file_autocomplete {
        if ac.selected_index + 1 < ac.matches.len() {
            ac.selected_index += 1;
        }
    }
}

KeyCode::Enter | KeyCode::Tab if self.file_autocomplete.is_some() => {
    if let Some(ref ac) = self.file_autocomplete {
        self.accept_file_completion(ac);
    }
    self.file_autocomplete = None;
}

KeyCode::Esc => {
    // Cancel autocomplete
    self.file_autocomplete = None;
}
```

### FASE 3: UI Component

#### 3.1. Criar `FileAutocompleteWidget` (`limit-tui/src/components/file_autocomplete.rs`)

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Widget},
};

pub struct FileAutocompleteWidget<'a> {
    matches: &'a [FileMatch],
    selected_index: usize,
    query: &'a str,
}

impl<'a> Widget for FileAutocompleteWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render popup list with:
        // - Highlighted query matches
        // - Selected item styled differently
        // - Max 5-10 items visible
        // - Scroll indicator if needed
    }
}
```

#### 3.2. Atualizar `draw_ui()` em `TuiApp`

```rust
// Após renderizar input area, renderizar popup se ativo
if let Some(ref ac) = self.file_autocomplete {
    if ac.is_active && !ac.matches.is_empty() {
        let popup_area = self.calculate_popup_area(input_area, ac.matches.len());
        let widget = FileAutocompleteWidget::new(&ac.matches, ac.selected_index, &ac.query);
        f.render_widget(widget, popup_area);
    }
}
```

### FASE 4: Features Adicionais

#### 4.1. Suporte a paths relativos

```rust
// @src/main.rs     → src/main.rs
// @../Cargo.toml   → ../Cargo.toml
// @./lib           → ./lib/
```

#### 4.2. Cache inteligente

```rust
pub struct FileFinder {
    cached_files: Vec<PathBuf>,
    last_scan: std::time::Instant,
    cache_ttl: std::time::Duration, // 5 segundos
}

impl FileFinder {
    pub fn get_files(&mut self) -> &[PathBuf] {
        if self.last_scan.elapsed() > self.cache_ttl {
            self.refresh_cache();
        }
        &self.cached_files
    }
}
```

#### 4.3. Integração com .gitignore

```rust
use glob::Pattern;

fn load_ignore_patterns(dir: &Path) -> Vec<Pattern> {
    let mut patterns = Vec::new();
    
    // .gitignore
    if let Ok(content) = std::fs::read_to_string(dir.join(".gitignore")) {
        for line in content.lines() {
            if let Ok(pattern) = Pattern::new(line) {
                patterns.push(pattern);
            }
        }
    }
    
    // .ignore (ripgrep style)
    // .limitignore (project specific)
    
    patterns
}
```

#### 4.4. Múltiplos arquivos

```rust
// Suportar @file1 @file2 @file3
// Manter lista de arquivos mencionados
pub struct MentionedFiles {
    files: Vec<PathBuf>,
}

impl TuiApp {
    fn extract_mentioned_files(&self, text: &str) -> Vec<PathBuf> {
        // Regex: @([^\s]+)
        // Validar se arquivo existe
    }
}
```

## Testes

### Unit Tests

```rust
#[test]
fn test_file_finder_basic() {
    let finder = FileFinder::new(PathBuf::from("."));
    let files = finder.scan_files(3);
    assert!(files.contains(&PathBuf::from("Cargo.toml")));
}

#[test]
fn test_fuzzy_matching() {
    let matches = finder.filter_files(&files, "Cargo");
    assert!(matches.iter().any(|m| m.path == PathBuf::from("Cargo.toml")));
}

#[test]
fn test_autocomplete_activation() {
    let mut app = TuiApp::new(/* ... */);
    app.handle_key_event(KeyCode::Char('@'));
    assert!(app.file_autocomplete.is_some());
}
```

### Integration Tests

```rust
#[test]
fn test_complete_file_flow() {
    // 1. Type @
    // 2. Type "Cargo"
    // 3. Press Down
    // 4. Press Enter
    // 5. Verify input contains "@Cargo.toml"
}
```

## Checklist de Implementação

- [ ] **FASE 1: Backend**
  - [ ] Adicionar dependências (frizbee, walkdir, glob)
  - [ ] Criar `FileFinder` com scan de diretórios
  - [ ] Implementar fuzzy matching com Frizbee
  - [ ] Suporte a .gitignore/.ignore
  - [ ] Cache de arquivos com TTL

- [ ] **FASE 2: Estado**
  - [ ] Adicionar `FileAutocompleteState` ao `TuiApp`
  - [ ] Detectar `@` como trigger
  - [ ] Implementar navegação (↑/↓/Enter/Tab/Esc)
  - [ ] Inserir arquivo selecionado no input

- [ ] **FASE 3: UI**
  - [ ] Criar `FileAutocompleteWidget`
  - [ ] Renderizar popup sobre input
  - [ ] Highlight do query nos matches
  - [ ] Indicador visual de seleção
  - [ ] Scroll para listas longas

- [ ] **FASE 4: Features**
  - [ ] Paths relativos (../, ./)
  - [ ] Múltiplos arquivos (@file1 @file2)
  - [ ] Diretórios com trailing /
  - [ ] Preview de arquivo (opcional)

- [ ] **FASE 5: Testes & Docs**
  - [ ] Unit tests para FileFinder
  - [ ] Integration tests para TUI
  - [ ] Atualizar README
  - [ ] Documentar no DEVELOPMENT_GUIDE.md

## Estimativa de Tempo

- **FASE 1:** 4-6 horas
- **FASE 2:** 3-4 horas
- **FASE 3:** 4-5 horas
- **FASE 4:** 3-4 horas
- **FASE 5:** 2-3 horas

**Total:** 16-22 horas

## Riscos e Mitigações

| Risco | Mitigação |
|-------|-----------|
| Performance em projetos grandes | Cache TTL + max_depth limitado + lazy loading |
| Conflito com @ em mensagens normais | Permitir escapar com \\@ ou Cancelar com Esc |
| Paths com espaços | Inserir com aspas ou usar path relativo sempre |
| Arquivos muito longos | Truncar exibição com ... |

## Exemplo de Uso Final

```
Input: Leia o arquivo @Cargo.toml e analise as dependências

[Popup aparece mostrando:]
┌─────────────────────────────┐
│ > Cargo.toml                │  ← selecionado
│   Cargo.lock                │
│   .cargo/config.toml        │
│   limit-cli/Cargo.toml      │
│   limit-tui/Cargo.toml      │
└─────────────────────────────┘

[Após Enter:]
Input: Leia o arquivo @Cargo.toml e analise as dependências
                       ^^^^^^^^^^ (inserido automaticamente)
```

## Próximos Passos

1. **Validar escolha da biblioteca fuzzy** - Testar Frizbee vs Nucleo
2. **Criar branch** - `feature/file-autocomplete`
3. **Implementar FASE 1** - Backend com FileFinder
4. **Testar em projeto real** - Validar performance
5. **Continuar com FASE 2-5**

## Referências

- **Frizbee:** https://lib.rs/crates/frizbee
- **Nucleo:** https://github.com/helix-editor/nucleo
- **fuzzy-matcher:** https://github.com/skim-rs/fuzzy-matcher
- **Ratatui examples:** https://ratatui.rs/examples/apps/user_input/
- **Similar implementation:** GitHub Copilot, Cursor IDE, Zed editor

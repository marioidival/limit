#!/bin/bash
set -euo pipefail

# ============================================
# Script de Release - Limit Agent
# ============================================
# Automatiza o processo de bump de versão, geração de changelog
# e preparação para release
#
# Uso:
#   ./scripts/release.sh [major|minor|patch]
#
# Exemplos:
#   ./scripts/release.sh patch    # 0.0.16 -> 0.0.17
#   ./scripts/release.sh minor    # 0.0.16 -> 0.1.0
#   ./scripts/release.sh major    # 0.0.16 -> 1.0.0
# ============================================

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Funções de log
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Função para mostrar o racional
show_rationale() {
    log_info "========================================"
    log_info "RACIONAL DO SCRIPT DE RELEASE"
    log_info "========================================"
    echo ""
    echo "Este script automatiza as seguintes operações:"
    echo ""
    echo "1. ✅ Validação do repositório"
    echo "   - Verifica se está em um repositório git"
    echo "   - Garante que working directory está limpo"
    echo "   - Certifica que está na branch trunk"
    echo ""
    echo "2. 📦 Bump de Versão em TODOS os crates"
    echo "   - Atualiza limit-cli/Cargo.toml"
    echo "   - Atualiza limit-agent/Cargo.toml"
    echo "   - Atualiza limit-llm/Cargo.toml"
    echo "   - Atualiza limit-tldr/Cargo.toml"
    echo "   - Atualiza limit-tui/Cargo.toml"
    echo "   - (Workspace Cargo.toml não tem versão)"
    echo ""
    echo "3. 📝 Geração de CHANGELOG.md"
    echo "   - Usa git-cliff para gerar changelog automático"
    echo "   - Baseado em commits desde o último tag"
    echo "   - Segue formato: ## [versão] - data"
    echo ""
    echo "4. 🏷️ Criação de Git Tag"
    echo "   - Tag formatada: v0.0.XX"
    echo "   - Anotada com mensagem de release"
    echo ""
    echo "5. 📋 Commit das Mudanças"
    echo "   - Commit único com version + changelog"
    echo "   - Segue Conventional Commits"
    echo ""
    echo "6. 🔨 Build de Valida��ão (Opcional)"
    echo "   - Executa 'cargo build --release'"
    echo "   - Garante que o projeto compila"
    echo ""
    echo "========================================"
    echo ""
}

# Verifica dependências
check_dependencies() {
    log_info "Verificando dependências..."

    local missing_deps=()

    if ! command -v git &> /dev/null; then
        missing_deps+=("git")
    fi

    if ! command -v git-cliff &> /dev/null; then
        missing_deps+=("git-cliff")
    fi

    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_error "Dependências faltando: ${missing_deps[*]}"
        echo ""
        echo "Instale git-cliff:"
        echo "  cargo install git-cliff"
        echo ""
        echo "Ou:"
        echo "  brew install git-cliff"
        exit 1
    fi

    log_success "Todas as dependências instaladas"
}

# Valida o estado do repositório
validate_repo() {
    log_info "Validando repositório..."

    # Verifica se é um repositório git
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        log_error "Não está em um repositório git"
        exit 1
    fi

    # Verifica working directory limpo
    if [ -n "$(git status --porcelain)" ]; then
        log_error "Working directory não está limpo. Faça commit ou stache antes."
        git status --short
        exit 1
    fi

    # Verifica branch atual
    local current_branch=$(git rev-parse --abbrev-ref HEAD)
    if [ "$current_branch" != "trunk" ]; then
        log_warn "Você não está na branch 'trunk' (atual: $current_branch)"
        read -p "Continuar mesmo assim? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Abortando"
            exit 0
        fi
    fi

    log_success "Repositório validado"
}

# Obtém versão atual
get_current_version() {
    # Pega versão do limit-cli (crate principal)
    grep -E '^version = "' limit-cli/Cargo.toml | sed 's/version = "\([^"]*\)"/\1/'
}

# Calcula nova versão
calculate_new_version() {
    local current=$1
    local bump_type=$2

    # Separa em major.minor.patch
    IFS='.' read -r major minor patch <<< "$current"

    case $bump_type in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        patch)
            patch=$((patch + 1))
            ;;
        *)
            log_error "Tipo de bump inválido: $bump_type (use: major, minor, patch)"
            exit 1
            ;;
    esac

    echo "${major}.${minor}.${patch}"
}

# Atualiza versão em um Cargo.toml
update_cargo_toml_version() {
    local file=$1
    local new_version=$2

    log_info "Atualizando $file"

    # Substitui a linha version = "..." no início da linha (não em dependencies)
    sed -i '' "s/^version = \"[^\"]*\"$/version = \"$new_version\"/" "$file"

    log_success "  $file -> $new_version"
}

# Atualiza dependências internas do workspace
update_workspace_dependencies() {
    local new_version=$1

    log_info "Atualizando dependências do workspace para $new_version..."

    # Atualiza dependências limit-* nos Cargo.toml
    for file in limit-cli/Cargo.toml limit-agent/Cargo.toml limit-tui/Cargo.toml limit-tldr/Cargo.toml; do
        if [ -f "$file" ]; then
            # Atualiza limit-llm dependency
            sed -i '' "s/limit-llm = { path = \"\.\.\/limit-llm\", version = \"[^\"]*\" }/limit-llm = { path = \"\.\.\/limit-llm\", version = \"$new_version\" }/g" "$file"
            # Atualiza limit-agent dependency
            sed -i '' "s/limit-agent = { path = \"\.\.\/limit-agent\", version = \"[^\"]*\" }/limit-agent = { path = \"\.\.\/limit-agent\", version = \"$new_version\" }/g" "$file"
            # Atualiza limit-tui dependency
            sed -i '' "s/limit-tui = { path = \"\.\.\/limit-tui\", version = \"[^\"]*\" }/limit-tui = { path = \"\.\.\/limit-tui\", version = \"$new_version\" }/g" "$file"
            # Atualiza limit-tldr dependency
            sed -i '' "s/limit-tldr = { path = \"\.\.\/limit-tldr\", version = \"[^\"]*\" }/limit-tldr = { path = \"\.\.\/limit-tldr\", version = \"$new_version\" }/g" "$file"
            log_success "  Dependências atualizadas em $file"
        fi
    done

    log_success "Dependências do workspace atualizadas"
}

# Atualiza todas as versões no workspace
update_all_versions() {
    local new_version=$1

    log_info "Atualizando versão para $new_version em todos os crates..."

    update_cargo_toml_version "limit-cli/Cargo.toml" "$new_version"
    update_cargo_toml_version "limit-agent/Cargo.toml" "$new_version"
    update_cargo_toml_version "limit-llm/Cargo.toml" "$new_version"
    update_cargo_toml_version "limit-tldr/Cargo.toml" "$new_version"
    update_cargo_toml_version "limit-tui/Cargo.toml" "$new_version"

    # Atualiza dependências internas do workspace
    update_workspace_dependencies "$new_version"

    log_success "Todas as versões atualizadas"
}

# Gera CHANGELOG.md
generate_changelog() {
    local new_version=$1
    local current_date=$(date +%Y-%m-%d)

    log_info "Gerando CHANGELOG.md..."

    # Verifica qual config do git-cliff usar
    if [ -f ".cliff.toml" ]; then
        local cliff_config="--config .cliff.toml"
    elif [ -f "cliff.toml" ]; then
        local cliff_config="--config cliff.toml"
    else
        log_warn "Config do git-cliff não encontrada, usando configuração padrão"
        local cliff_config=""
    fi

    # Cria header temporário
    local temp_changelog=$(mktemp)
    echo "## [$new_version] - $current_date" > "$temp_changelog"
    echo "" >> "$temp_changelog"

    # Adiciona changelog gerado (se houver commits desde o último tag)
    if git tag | grep -q "^v"; then
        git cliff $cliff_config --tag "$new_version" --unreleased --prepend "$temp_changelog"
    else
        git cliff $cliff_config --tag "$new_version" --prepend "$temp_changelog"
    fi

    # Se o CHANGELOG já existe, adiciona o conteúdo antigo depois
    if [ -f "CHANGELOG.md" ]; then
        # Pula o header inicial do CHANGELOG existente (primeiras linhas até o primeiro ##)
        tail -n +3 CHANGELOG.md >> "$temp_changelog"
    fi

    # Move o temp para o local definitivo
    mv "$temp_changelog" CHANGELOG.md

    log_success "CHANGELOG.md gerado"
}

# Cria tag git
create_git_tag() {
    local new_version=$1

    local tag_name="v${new_version}"

    log_info "Criando tag git: $tag_name"

    # Verifica se tag já existe
    if git rev-parse "$tag_name" >/dev/null 2>&1; then
        log_error "Tag $tag_name já existe"
        exit 1
    fi

    # Cria tag anotada
    git tag -a "$tag_name" -m "Release $tag_name"

    log_success "Tag $tag_name criada"
}

# Faz commit das mudanças
commit_changes() {
    local new_version=$1
    local tag_name="v${new_version}"

    log_info "Fazendo commit das mudanças..."

    git add limit-cli/Cargo.toml
    git add limit-agent/Cargo.toml
    git add limit-llm/Cargo.toml
    git add limit-tldr/Cargo.toml
    git add limit-tui/Cargo.toml
    git add CHANGELOG.md
    git add Cargo.lock

    git commit -m "chore(release): bump version to $tag_name" \
                -m "" \
                -m "- Update all crate versions to $new_version" \
                -m "- Update CHANGELOG.md"

    log_success "Commit criado"
}

# Build de validação
validate_build() {
    log_info "Executando build de validação..."

    if cargo build --release; then
        log_success "Build validado com sucesso"
    else
        log_error "Build falhou!"
        log_error "Abortando release. Revise os erros acima."
        exit 1
    fi
}

# Main function
main() {
    # Mostra rationale se for solicitado
    if [ "${1:-}" == "--rationale" ] || [ "${1:-}" == "-r" ]; then
        show_rationale
        exit 0
    fi

    # Mostra help se solicitado ou sem argumentos
    if [ "${1:-}" == "--help" ] || [ "${1:-}" == "-h" ] || [ $# -eq 0 ]; then
        echo "Script de Release - Limit Agent"
        echo ""
        echo "Uso:"
        echo "  $0 [major|minor|patch]"
        echo ""
        echo "Opções:"
        echo "  --rationale, -r    Mostra o racional do script"
        echo "  --help, -h         Mostra esta mensagem de ajuda"
        echo ""
        echo "Exemplos:"
        echo "  $0 patch    # 0.0.16 -> 0.0.17"
        echo "  $0 minor    # 0.0.16 -> 0.1.0"
        echo "  $0 major    # 0.0.16 -> 1.0.0"
        exit 0
    fi

    local bump_type=$1

    # Valida tipo de bump
    if [[ ! "$bump_type" =~ ^(major|minor|patch)$ ]]; then
        log_error "Tipo inválido: $bump_type"
        echo "Use: major, minor ou patch"
        exit 1
    fi

    echo ""
    log_info "========================================"
    log_info "  RELEASE SCRIPT - Limit Agent"
    log_info "========================================"
    echo ""

    # Executa o fluxo
    check_dependencies
    validate_repo

    local current_version=$(get_current_version)
    local new_version=$(calculate_new_version "$current_version" "$bump_type")

    echo ""
    log_info "Versão atual:    $current_version"
    log_info "Tipo de bump:    $bump_type"
    log_info "Nova versão:     $new_version"
    echo ""

    # Confirmação
    read -p "Continuar com o release? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Abortando"
        exit 0
    fi

    echo ""

    update_all_versions "$new_version"
    
    # Atualiza o Cargo.lock para refletir as novas versões
    log_info "Atualizando Cargo.lock..."
    cargo generate-lockfile
    log_success "Cargo.lock atualizado"
    
    generate_changelog "$new_version"
    commit_changes "$new_version"
    create_git_tag "$new_version"

    echo ""
    log_success "========================================"
    log_success "  Release preparado com sucesso!"
    log_success "========================================"
    echo ""
    log_info "Próximos passos:"
    echo "  1. Revise o commit:   git log -1 --stat"
    echo "  2. Revise o changelog: cat CHANGELOG.md | head -30"
    echo "  3. Faça push:         git push origin trunk"
    echo "  4. Push da tag:       git push origin $new_version"
    echo ""

    # Pergunta se quer fazer build
    read -p "Executar build de validação? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        validate_build
    fi

    echo ""
    log_success "✨ Release $new_version pronto para push!"
}

# Executa main
main "$@"

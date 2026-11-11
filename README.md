# Cherry Browser — Alpha 0.4.12

Navegador para Windows com interface em Rust + Tauri e páginas em abas do WebView2 do Microsoft Edge (Chromium). Este projeto está em **ALPHA**.

## Baixar e instalar

Execute `Cherry-Browser-Setup.exe`. O instalador abre uma tela gráfica nativa do Windows, sem terminal, instala em `%LOCALAPPDATA%\Programs\Cherry Browser`, cria o atalho do Menu Iniciar e permite criar o atalho da Área de Trabalho. Não exige administrador. Clique em “Concluir e abrir” para iniciar o navegador. O Windows precisa do runtime WebView2 Evergreen.

## Recursos

- Abas dinâmicas, restauração das últimas abas comuns, navegação, favoritos (com remoção pela lista), histórico e downloads
- Indicador de carregamento com botão para interromper a página
- Atalhos Ctrl+T, Ctrl+Shift+T, Ctrl+Shift+N, Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+R, Ctrl+L e Ctrl+W
- Mini-player Spotify flutuante para links públicos sem iniciar um fluxo obrigatório de login
- Speedtest da Ookla abre como uma página nativa em nova guia
- DNS sobre HTTPS: Cloudflare por padrão, Google Public DNS, Quad9 ou DNS da rede (reinício necessário)
- Atalhos de Favoritos e bloqueador de anúncios junto ao botão de downloads
- Verificador de versões Alpha em Releases do GitHub, com acesso direto às versões publicadas
- O verificador em Sobre baixa e verifica o instalador de um release público e aplica a atualização automaticamente, reabrindo o navegador
- Google, Microsoft Bing (Edge), Brave Search e DuckDuckGo
- Bloqueio nativo de requisições de domínios conhecidos de anúncios e rastreadores, com ocultação de elementos comuns e controle imediato
- Personalização de tema claro, escuro ou noturno, capa de cerejas e escolha de inicialização (nova guia ou restaurar sessão)
- Modo privado e atalhos Ctrl+T, Ctrl+L e Ctrl+W
- Menu que se sobrepõe à página sem deslocá-la; ao fechar, restaura a área completa da aba
- Instalador gráfico por usuário, sem janela de terminal

## Limites do Alpha

O bloqueador usa regras básicas de domínios e elementos; não tem a cobertura do Brave Shields. Extensões, sincronização e VPN não estão incluídas. Favoritos e histórico ficam neste dispositivo. O instalador não tem assinatura digital.

## Versões e builds

`VERSION.json` é a fonte da versão e do canal. O build confere se Cargo, Tauri e npm correspondem ao manifesto, compila o código-fonte atual, publica um executável com versão no nome, substitui os arquivos de release anteriores em `outputs` e preserva a pasta do código-fonte.

## Recompilar no Windows

Requisitos: Node.js/npm, Rust MSVC, Visual Studio Build Tools com Desenvolvimento para desktop com C++, Windows SDK, compilador C# do .NET Framework e WebView2 Runtime.

```powershell
npm install
powershell -ExecutionPolicy Bypass -File .\build-installer.ps1
```


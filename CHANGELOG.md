## 0.4.12-alpha — 2026-11-10

- O bloqueador agora interrompe requisições conhecidas de anúncios e rastreadores no WebView2 antes do download, com liga/desliga imediato por aba e regras visuais ampliadas.

## 0.4.11-alpha — 2026-11-10

- Restaurado o atalho direto para o Speedtest da Ookla em uma nova guia.

## 0.4.10-alpha — 2026-11-10

- Removido o botão que iniciava o fluxo de login do Spotify; links públicos podem ser abertos sem entrar, e a conta só é usada se o próprio Spotify solicitar.

## 0.4.9-alpha — 2026-11-10

- O atalho de teste de velocidade agora pesquisa usando o mecanismo padrão escolhido nas configurações, incluindo Brave Search.

## 0.4.8-alpha — 2026-11-10

- Corrigida a ordem visual do WebView nativo do Spotify para o player aparecer sobre a página.
- O mini-player informa quando o embed termina de carregar e continua usando o perfil normal do Cherry.
- Adicionado Speedtest da Ookla como uma página nativa em uma nova guia.
- Adicionada escolha de DNS sobre HTTPS: Cloudflare por padrão, Google, Quad9 ou DNS da rede. A troca pode ser aplicada reiniciando o navegador.

## 0.4.7-alpha — 2026-11-10

- O Spotify agora abre em uma WebView própria sobre o mini-player, no perfil compartilhado do Cherry, sem depender de cookies de iframe de terceiros.
- O mini-player reutiliza a sessão iniciada no Web Player do Spotify em uma guia do Cherry.

## 0.4.6-alpha — 2026-11-10

- Ao abrir o mini-player, o item Spotify salvo é carregado novamente para usar uma sessão autenticada no Cherry.
- O login é feito no site oficial em uma guia do navegador; não exige Client ID nem importa credenciais de outros navegadores.

## 0.4.5-alpha — 2026-11-10

- O mini-player do Spotify ganhou um botão para abrir o Spotify e entrar na conta em uma nova guia.
- Personalização permite escolher entre iniciar com uma nova guia ou restaurar as guias da sessão anterior.

# Histórico de versões

## 0.4.4-alpha — 2026-11-10

- Em Sobre, o verificador agora oferece o download direto do instalador quando encontra uma versão pública mais recente.
- O estado diferencia uma versão instalada ainda não publicada de uma versão pública atualizada.

## 0.4.3-alpha — 2026-11-10

- Adicionado mini-player flutuante do Spotify junto aos botões de favoritos, downloads e bloqueador.
- Player aceita links e URIs de faixa, álbum, playlist, artista, episódio ou podcast e lembra o último item usado.
- O popup recorta apenas a área correspondente da página nativa para aparecer sobre ela sem fechar a navegação.

## 0.4.2-alpha — 2026-11-10

- As últimas abas HTTP/HTTPS abertas são restauradas ao iniciar; abas privadas nunca entram na sessão salva.
- Adicionados Ctrl+R, Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+Shift+N e Ctrl+Shift+T, inclusive com foco dentro de páginas.
- O verificador em Sobre consulta os Releases públicos do GitHub, compara versões e abre a página da versão disponível.
- Restaurar uma sessão não duplica as páginas no Histórico.
- A barra de ferramentas mostra quando a página carrega e permite interromper o carregamento.
- Favoritos podem ser removidos diretamente pela lista de gerenciamento.

## 0.4.1-alpha — 2026-11-09

- Adicionado “Verificar atualizações” em Sobre; informa que a checagem remota depende de um canal público de releases.
- Favoritos e controle do bloqueador agora ficam junto a Downloads na barra.
- O botão de nova aba acompanha as abas e rola para o fim da faixa conforme a lista cresce.
- Melhorado o estado visual do bloqueador e o acesso rápido a Favoritos.

## 0.4.0-alpha — 2026-11-09

- Atalhos Ctrl+T, Ctrl+L e Ctrl+W interceptados pelo WebView2 mesmo quando o site está focado.
- Limpeza de dados também funciona sem abas de navegação abertas; remove histórico local, cookies e cache do perfil do navegador.
- Estrela da barra de endereço indica favoritos salvos usando a cor do tema e alterna adicionar/remover.
- Corrigida a versão de reserva mostrada pela interface.
- Ampliados os domínios e seletores usados pelo bloqueador básico de anúncios.

## 0.3.9-alpha — 2026-11-09

- Adicionada a assinatura “by Leo e Vini” à seção Sobre.

## 0.3.8-alpha — 2026-11-09

- Corrigida a restauração do recorte do menu ao abrir painéis e alternar abas.
- Corrigido o retorno à navegação ao sair de um painel pelo endereço ou pelo botão Início.
- O bloqueador de anúncios agora aplica e remove sua folha de estilos ao alternar a opção.
- Agrupada a inspeção de elementos novos para reduzir varreduras repetidas em páginas dinâmicas.
- Reduzidas chamadas nativas durante o redimensionamento e a troca de abas; listas locais inválidas agora são ignoradas com segurança.

## 0.3.7-alpha — 2026-11-09

- Menu do navegador agora se sobrepõe à página na posição atual, sem redimensioná-la ou movê-la.
- A área sob o popup é liberada pela janela nativa da página e restaurada ao fechar o menu.

## 0.3.6-alpha — 2026-11-09

- Implementação experimental de menu flutuante; substituída na 0.3.7 por recorte nativo localizado da página.

## 0.3.5-alpha — 2026-11-09

- Removido o painel cinza do fundo do menu; a barra e a página ficam visíveis.
## 0.3.4-alpha — 2026-11-09

- Refinado o fundo do menu para manter os controles da barra legíveis.
- Ajustados os botões do tema Claro para evitar cartões rosa muito fortes.

## 0.3.3-alpha — 2026-11-09

- Corrigido o ícone Windows do executável com a marca de cereja do Cherry Browser.
- Executável compilado como aplicativo gráfico do Windows, sem janela de terminal.
- Menu mantém a página aberta visível e reserva espaço próprio enquanto está aberto.
- Refeitos os temas Cereja, Claro e Noite com superfícies e contraste consistentes.

## 0.3.2-alpha — 2026-11-09

- Corrigido o menu de três pontos: páginas WebView2 são escondidas enquanto o menu está aberto e restauradas ao fechar.
- Marcado o app como ALPHA na página inicial, na seção Sobre e no instalador.
- Centralizada a versão e o canal em `VERSION.json`; o build verifica o manifesto antes de compilar.
- O build troca os executáveis e ZIPs da versão anterior e preserva o código-fonte.

## 0.3.1 — 2026-11-09

- Removidos os controles de janela duplicados; minimizar, maximizar e fechar ficam na barra nativa do Windows.
- Aumentado o tamanho mínimo da janela para manter a interface legível.

## 0.3.0 — 2026-11-09

- Adicionada personalização de tema e capa de cerejas para a página inicial.
- Adicionado bloqueio básico de anúncios e rastreadores conhecidos, com controle nas configurações.
- Adicionadas opções Google, Microsoft Bing (usado pelo Edge), Brave Search e DuckDuckGo.
- Instalador recebeu tela gráfica nativa, instalação por usuário sem terminal e opção de atalho na Área de Trabalho.
- Ícone, favicon e capa próprios com a fruta cereja.

## 0.2.0 — 2026-11-09

- Renomeado de Leve Browser para Cherry Browser.
- Corrigido o carregamento da interface com Vite; os módulos Tauri agora são empacotados corretamente.
- Adicionadas abas WebView2, navegação, busca, favoritos, histórico, downloads, modo privado e configurações.




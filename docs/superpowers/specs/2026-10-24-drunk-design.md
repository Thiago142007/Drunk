# Drunk — especificação do projeto

**Status:** proposta para revisão e aprovação; nenhuma implementação foi iniciada.  
**Data:** 2026-10-24  
**Plataforma inicial:** Linux desktop x86-64  
**Linguagem do código próprio:** Rust

## 1. Intenção

Criar o **Drunk**, um projeto comunitário e não oficial para permitir que usuários Linux iniciem experiências Roblox por meio do cliente Android oficial, dentro de um runtime especializado para desktop Linux. O projeto é inspirado no resultado observado do Sober, mas não depende de código privado, não copia implementação desconhecida e não se apresenta como afiliado ao Roblox ou ao VinegarHQ.

A opção escolhida é **B**: desenvolver uma camada/runtime próprio do lado Linux em Rust, com componentes Android externos explicitamente delimitados. “Em Rust” significa que o código mantido pelo Drunk será escrito em Rust. Não significa reimplementar o Android, o Roblox, drivers ou bibliotecas nativas; o APK oficial e dependências de sistema são executados/consumidos como componentes externos.

## 2. Pesquisa sobre o Sober: fatos e limites

O repositório público `vinegarhq/sober` diz que o projeto é fechado e que o GitHub serve como rastreador de problemas e sugestões. Portanto, não há código-fonte público suficiente para inspecionar os módulos internos, reproduzir sua implementação ou confirmar detalhes da ponte Android/Linux.

A documentação oficial declara que o Sober cria um runtime especializado para a versão Android do Roblox e faz a ponte entre Android e Linux. Ele instala como Flatpak e, no primeiro uso, solicita a instalação do Roblox, automaticamente ou por seleção manual de um bundle APK. A documentação expõe opções de gráficos, input por toque/gamepad, interface, HiDPI, integração com Discord, localização de servidor e armazenamento de sessão.

A documentação de instalação lista Linux kernel 5.11 ou superior, x86-64, SSE4.1 e OpenGL ES 3.0 como referências mínimas; recomenda SSE4.2 e Vulkan 1.1. A própria documentação alerta para limitações de compatibilidade, problemas de GPU/driver e experiências que exigem Android Remote Attestation. Essas referências são um ponto de comparação para a pesquisa do Drunk, não uma promessa de compatibilidade do Drunk.

### Fatos confirmados versus inferências

| Afirmação | Estado | Base |
|---|---|---|
| Sober usa a versão Android do Roblox em um runtime adaptado a Linux | Confirmado pela documentação do projeto | FAQ e página inicial |
| Sober é fechado e o repo público é para issues | Confirmado pelo README do repo | GitHub `vinegarhq/sober` |
| Sober usa um conjunto específico de bibliotecas ou técnicas internas | Desconhecido | Código interno não publicado |
| Drunk pode obter compatibilidade equivalente apenas por ser escrito em Rust | Não demonstrado | Exige prova técnica; linguagem não fornece APIs Android nem compatibilidade automaticamente |
| Um runtime Linux próprio pode executar o APK sem ambiente Android completo | Pergunta técnica em aberto | Deve ser validada antes de definir a arquitetura final |

Não usar especulação sobre Sober como fato. Toda descoberta posterior deve registrar fonte, data e grau de confiança.

## 3. Objetivos

1. Investigar e construir, em etapas, um runtime Linux para iniciar o cliente Android oficial do Roblox em desktop Linux.
2. Manter em Rust todo o código próprio do projeto: supervisor/runtime, configuração, CLI, diagnósticos e integrações que sejam viáveis com interfaces documentadas.
3. Propor uma instalação previsível, isolada e removível, com Flatpak como formato de distribuição inicial a avaliar.
4. Validar compatibilidade de CPU, kernel, GPU e driver antes de tentar iniciar o cliente.
5. Permitir instalação do cliente por uma fonte de confiança e/ou seleção manual de bundle, respeitando licenças e termos aplicáveis.
6. Oferecer logs úteis, mensagens de erro acionáveis, configuração documentada e recuperação simples do estado do Drunk.
7. Tratar login, sessão e arquivos do usuário como dados sensíveis.

## 4. Não objetivos

- Reimplementar o Roblox, Roblox Studio, Android, ART, Bionic, Vulkan, OpenGL ou drivers em Rust.
- Incluir, distribuir ou modificar binários proprietários do Roblox sem autorização apropriada.
- Desenvolver um cliente alternativo, protocolo de rede ou mecanismo para evitar controles da plataforma.
- Bypassar anti-cheat, verificação de integridade, Remote Attestation, DRM, bloqueios de plataforma ou controles de segurança.
- Suportar Roblox Studio nesta primeira etapa.
- Prometer compatibilidade com todas as experiências, GPUs, distribuições, arquiteturas ARM, VR, múltiplas instâncias ou ambientes virtuais.
- Reproduzir marcas, interface ou identidade visual do Sober de forma que implique afiliação.

## 5. Abordagens consideradas

### A. Drunk como integração com Waydroid ou outro ambiente Android

Menor investimento em runtime próprio e acesso a um Android completo existente. Em contrapartida, o produto depende de configuração e ciclo de vida de outra plataforma, o que diverge do objetivo B.

### B. Runtime Linux próprio do Drunk em Rust, com dependências Android externas — escolhida

O Drunk controla ciclo de vida, configuração, validação de ambiente, instalação/seleção do pacote, integração desktop e diagnóstico. Componentes Android e o cliente Roblox permanecem dependências externas. Essa direção está mais próxima do objetivo do usuário e do funcionamento descrito publicamente para Sober.

O custo e a viabilidade desta abordagem ainda não estão provados. A primeira etapa deve ser um protótipo limitado para responder se existe uma composição de componentes documentados/licenciáveis que inicia o cliente Android escolhido em Linux sem reimplementar partes enormes do Android.

### C. Recriar Android e cliente em Rust

Descartada: recriar Android e o cliente Roblox em Rust não é um plano de implementação realista, não é necessário para que o código do Drunk seja Rust e envolveria software proprietário que o projeto não controla.

## 6. Princípios de arquitetura propostos

A arquitetura detalhada depende da prova técnica inicial. O desenho pretendido separa as responsabilidades abaixo, sem pressupor que sejam os módulos usados pelo Sober:

1. **CLI/supervisor Rust:** valida argumentos e configuração, verifica dependências, inicia e acompanha processos filhos, encaminha sinais e coleta logs. Deve encerrar processos filhos corretamente e não deixar instâncias zumbis.
2. **Gerenciador de estado/configuração Rust:** mantém dados do Drunk sob diretório de usuário previsível, valida configuração versionada e oferece reset documentado. Segredos não devem ser salvos em texto puro por padrão.
3. **Instalador/validador de pacote:** aceita apenas arquivos esperados, valida estrutura, arquitetura e integridade; não instala conteúdo arbitrário silenciosamente. O caminho automático só será implementado se houver fonte e método de distribuição permitidos, estáveis e verificáveis.
4. **Adaptador de runtime Android/Linux:** fronteira explícita com dependências externas. A prova técnica deve comparar runtime Android próprio construído com componentes existentes versus composição de baixo nível; a decisão deve considerar suporte, licenças, segurança e manutenção.
5. **Ponte de janela, input, áudio e gráficos:** somente será especificada após demonstrar como o runtime escolhido expõe essas APIs. Não inventar compatibilidade usando FFI não documentada.
6. **Integração desktop:** arquivo `.desktop`, associação de links `roblox-player` via mecanismos suportados pela distribuição/Flatpak e seletor de arquivos apropriado.
7. **Empacotamento:** Flatpak é candidato inicial por isolamento e reprodutibilidade, em linha com a distribuição do Sober. As permissões devem ser mínimas e explicadas; não assumir acesso amplo ao sistema de arquivos.

### Regra de linguagem

Código escrito e mantido no repositório Drunk será Rust. Bindings Rust para bibliotecas C do sistema podem existir quando forem inevitáveis, mas suas dependências nativas devem ser listadas; isso não equivale a código próprio escrito em C/C++. Binários do Android/Roblox não contam como código do Drunk e não devem ser alterados pelo projeto.

## 7. Escopo inicial e etapas

### Etapa 0 — viabilidade técnica (bloqueadora)

- Definir um método permitido de obter um pacote oficial e verificável.
- Identificar interfaces Android necessárias pelo comportamento documentado e por ferramentas públicas, sem depender de engenharia reversa de código fechado do Sober.
- Comparar opções de runtime e confirmar licenças, manutenção e integração com Linux.
- Provar em uma máquina x86-64 Linux um caminho mínimo: detectar ambiente, preparar o runtime, abrir uma janela e iniciar uma aplicação Android de teste não Roblox.
- Produzir uma decisão arquitetural com dependências, limitações e riscos.
- **Critério de saída:** existe um caminho demonstrado, documentável e compatível com as restrições do projeto. Se não existir, parar e propor nova direção antes de criar o produto.

### Etapa 1 — esqueleto Rust

- CLI com `doctor`, `config`, `logs` e `run` em estado inicial.
- Detecção de kernel, arquitetura, instruções da CPU, drivers e API gráfica.
- Configuração tipada e migrações versionadas.
- Supervisor de processos com encerramento e propagação de sinais.
- Logs locais com rotação/limites e remoção de segredos.

### Etapa 2 — runtime e aplicação

- Instalação ou seleção manual de bundle, validação e mensagens de erro.
- Inicialização do runtime externo escolhido e coleta de logs diagnósticos.
- Janela, input, áudio e renderização conforme o protótipo comprovar.
- Integração de links de experiência e desktop.

### Etapa 3 — distribuição e compatibilidade

- Flatpak com permissões mínimas e manifest reproduzível.
- Matriz de validação em hardware AMD/Intel/NVIDIA e Wayland/X11, conforme disponibilidade.
- Guia de instalação, privacidade, limitações, diagnóstico e remoção completa de dados.
- Política de atualização e resposta quando o Roblox muda o APK ou APIs necessárias.

## 8. Requisitos funcionais propostos

- **RF-01:** `drunk doctor` apresenta requisitos detectados e explica falhas sem tentar contorná-las.
- **RF-02:** `drunk config` permite consultar/alterar opções suportadas sem edição manual insegura.
- **RF-03:** `drunk install` aceita um bundle compatível por fonte verificada ou arquivo escolhido pelo usuário, conforme a decisão da Etapa 0.
- **RF-04:** `drunk run` inicia a aplicação, informa progresso e encerra processos filhos quando o usuário fecha a sessão.
- **RF-05:** logs permitem diagnosticar falhas sem armazenar senha, cookie de sessão ou tokens.
- **RF-06:** `drunk reset` explica o que será removido e permite limpar estado/configuração do Drunk.
- **RF-07:** links de experiência são encaminhados ao Drunk por associação desktop configurável.
- **RF-08:** opção gráfica alternativa (por exemplo Vulkan/OpenGL) só aparece se o backend validado suportar ambas.

## 9. Requisitos de qualidade e segurança

- **Rust:** código próprio em Rust estável; versões e dependências fixadas/revisáveis.
- **Isolamento:** minimizar permissões de filesystem, dispositivos e rede no pacote; justificar cada permissão.
- **Integridade:** verificar origem e hash de downloads quando houver metadado confiável; rejeitar bundles incompletos ou arquitetura incompatível.
- **Segredos:** não imprimir cookies, credenciais ou tokens; usar armazenamento de segredo do sistema quando suportado; documentar qualquer fallback.
- **Atualizações:** tratar falha de rede, atualização parcial e downgrade incompatível sem corromper estado do usuário.
- **Operação:** nenhuma função de multi-instância por padrão; encerramento limpo; logs limitados.
- **Transparência:** avisar que é não oficial e experimental; nunca prometer que o serviço ou uma experiência específica funcionará.

## 10. Limitações e riscos

| Risco | Impacto | Resposta proposta |
|---|---|---|
| Sober é fechado | Não é possível reproduzir seus detalhes internos com fidelidade | Basear-se em documentação pública e medir o próprio protótipo |
| Runtime Android não definido | Pode invalidar arquitetura e cronograma | Bloquear implementação de produto na Etapa 0 |
| Atualizações frequentes do Roblox | Quebra de compatibilidade ou instalador | Versionar compatibilidade e falhar com diagnóstico claro |
| Binários/APKs proprietários | Distribuição indevida e questões de licença | Não redistribuir sem autorização; usar fluxo compatível e transparente |
| Controles de integridade/attestation | Algumas experiências podem recusar conexão | Não tentar burlar; explicar incompatibilidade |
| Diversidade de GPUs/drivers | Crashes, erros de memória ou renderização | `doctor`, backend alternativo validado e matriz de compatibilidade |
| Sessão/autenticação | Exposição de conta do usuário | Minimizar armazenamento, mascarar logs, armazenar segredo com proteção do sistema |
| Dependências C do sistema | Pode contrariar interpretação literal de “100% Rust” | Manter lógica do Drunk em Rust e publicar inventário de bindings e bibliotecas |

## 11. Decisões e questões abertas para a implementação

Decidido pelo usuário:

- Nome de trabalho: **Drunk**.
- Plataforma-alvo: Linux.
- Linguagem do código próprio: Rust.
- Direção de arquitetura: runtime Linux próprio com componentes Android externos (opção B).

Propostas do documento, ainda sujeitas à aprovação:

- Começar por x86-64 desktop e avaliar Flatpak primeiro.
- Tratar prova técnica como etapa bloqueadora.
- Não dar suporte a Studio, ARM, VR, múltiplas instâncias ou bypass de segurança na primeira versão.
- O implementador poderá escolher crates, CLI/UI e detalhes internos após a prova técnica, desde que preserve os limites acima.

Questões que a implementação deve resolver com evidência:

1. Qual runtime Android pode ser integrado e distribuído de forma sustentável?
2. Como obter e validar o bundle Roblox sem redistribuir conteúdo proprietário?
3. Quais interfaces gráficas e de input estarão disponíveis no runtime escolhido?
4. O usuário quer uma CLI inicial, uma interface gráfica, ou ambas? Default sugerido para o protótipo: CLI; a interface final deve ser decidida após validar o runtime.

## 12. Referências consultadas

Consultadas em 2026-10-24; documentação de terceiros pode mudar.

- [Sober — site oficial](https://sober.vinegarhq.org/): descreve Sober como runtime especializado para a versão Android do Roblox e avisa que é experimental, não oficial e fechado.
- [Sober — repositório público](https://github.com/vinegarhq/sober): informa que o repo é issue tracker e contém README/issue tracking, sem implementação pública.
- [Sober — FAQ](https://vinegarhq.org/Sober/FAQ/index.html): base Android, status não oficial, limites e notas sobre arquitetura.
- [Sober — instalação e requisitos](https://vinegarhq.org/Sober/Installation.html): instalação Flatpak, fluxo de primeiro uso e requisitos publicados.
- [Sober — configuração](https://vinegarhq.org/Sober/Configuration/index.html): opções de gráficos, input, UI, HiDPI, sessão e integração.
- [Sober — troubleshooting](https://vinegarhq.org/Sober/Troubleshooting.html): logs, drivers, Vulkan/OpenGL, APKs, portais, Remote Attestation e limites conhecidos.
- [Flathub — Sober](https://flathub.org/en/apps/org.vinegarhq.Sober): pacote e avisos de distribuição; usado como referência secundária sobre a distribuição.

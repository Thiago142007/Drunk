# ADR 0001 — Backend Android: no-go provisório

**Data:** 2026-10-24. **Decisão:** **NO-GO provisório para iniciar o runtime/produto da opção B. Backend selecionado: nenhum.**

**Bloqueadores visíveis:** não há composição B executável demonstrada, inventário de licenças de uma composição selecionada ou lançamento manual de app Android não Roblox. Gráficos, input, áudio e encerramento permanecem sem medição. A conclusão é falta de evidência suficiente, não impossibilidade comprovada.

## Contexto

A [especificação](../superpowers/specs/2026-10-24-drunk-design.md), seções 5–7, escolhe runtime Linux próprio em Rust com componentes Android externos (B) e impõe prova técnica antes do produto. A [avaliação](../runtime-evaluation.md) contém fontes, comandos da inspeção local, comparação e protocolo futuro.

O [README público do Sober](https://github.com/vinegarhq/sober) identifica um issue tracker; o [site](https://sober.vinegarhq.org/) informa que o projeto é fechado; a [FAQ](https://vinegarhq.org/Sober/FAQ/index.html) descreve um runtime para o cliente Android. Essas fontes não fornecem um contrato de integração nem permitem afirmar quais componentes internos ele usa.

## Alternativas e decisão

| Alternativa | Decisão e fundamento |
| --- | --- |
| Adaptador próprio com componentes AOSP | Preserva conceitualmente B. A [arquitetura AOSP](https://source.android.com/docs/core/architecture) e a [ABI x86_64](https://developer.android.com/ndk/guides/abis) estão documentadas, mas a integração mínima Linux não foi demonstrada. Licenças devem ser verificadas por componente; [AOSP não usa uma única licença](https://source.android.com/docs/setup/contribute/licenses). Não selecionar agora. |
| Waydroid como backend externo completo | [Waydroid documenta Android completo em container com namespaces](https://docs.waydro.id/). **Avaliação do projeto:** um supervisor Rust que apenas usa seu CLI corresponde à opção A, não comprova B. Manter como referência; exigir nova decisão do usuário para adotá-lo. |
| Android completo em VM, representado pelo Android Emulator | O [emulador oficial](https://developer.android.com/studio/run/emulator) fornece um dispositivo Android virtual. **Avaliação do projeto:** é possível controle de laboratório, não prova do adaptador B. [Termos do SDK](https://developer.android.com/studio/terms) e licenças dos artefatos precisam ser resolvidos antes de distribuição. Não selecionar agora. |

A inspeção de leitura encontrou ADB e Sober instalado, mas não encontrou Waydroid/emuladores no PATH e nos caminhos adicionais registrados na avaliação. Nenhum app Android foi iniciado; não houve instalação, download de APK, alteração de serviço ou configuração. Portanto, o critério obrigatório de lançamento manual não foi satisfeito. A instalação do Sober não preenche esse critério para um runtime Drunk próprio.

## Primeiro marco publicado para revisão

Em 2026-10-24, o marco conserva a decisão e os bloqueadores acima. A [seção de diagnóstico local](../runtime-evaluation.md#31-diagnóstico-de-leitura-do-primeiro-marco) registra os resultados de leitura da tarefa 3 (`dd4b143`): Linux x86-64, kernel 7.0, SSE4.1/SSE4.2 presentes, dois nós DRI e arquivos de loaders Vulkan/EGL/GL presentes. São apenas indícios de capacidade da máquina; não demonstram APIs gráficas, permissões, funcionamento de drivers ou compatibilidade Android/Roblox. RAM disponível, compositor/versão e requisitos da composição futura continuam sem verificação.

O host foi inspecionado para o protocolo futuro; nenhum lançamento Android foi realizado. A composição B e um runtime para os samples não foram selecionados/preparados, os samples não foram preparados e não existe resultado de execução. O gate falhou nessa preparação, antes de qualquer medição de janela, input, áudio, gráficos ou parada.

O [resumo do marco](../runtime-evaluation.md#7-primeiro-marco-para-revisão) está pronto para revisão do usuário. A etapa termina aqui: não iniciar runtime Roblox nem elaborar o plano de produto até a viabilidade sustentar B. Esse plano posterior deverá cobrir origem e instalação permitida dos pacotes, renderização/input/áudio, integração desktop, Flatpak, instalador completo e ciclo de vida de sessões de jogo. Adoção de Waydroid como está ou mudança para outro Android completo exige nova decisão de arquitetura do usuário; nenhuma foi tomada neste marco.

## Consequências e riscos não resolvidos

- Pausar runtime de produto, pontes, instalador e qualquer alegação de compatibilidade. A investigação documental desta tarefa está concluída; o gate de execução continua reprovado.
- O custo de manter interfaces Android/Linux próprias permanece desconhecido. É uma **inferência de risco** decorrente da composição ainda indefinida.
- GPU/driver, privilégios, IPC, áudio e ciclo de vida não estão validados. A presença de Mesa ou `/dev/kvm` não substitui medição.
- Licenças de código aberto, imagens, SDK e aplicativos têm fronteiras distintas; nenhuma permissão de redistribuir Roblox ou serviços proprietários foi estabelecida.
- Um sample funcionar não garante Roblox, login, attestation ou experiências. Permanecem as restrições da especificação contra alteração do cliente e evasão de controles.

## Reabertura

Reavaliar esta ADR quando houver componentes e versões fixadas, APIs públicas suficientes, inventário de licenças/distribuição e responsável por manutenção, junto com evidência do [protocolo manual](../runtime-evaluation.md#5-protocolo-manual-futuro--não-executado): app não Roblox em janela, gráficos/input/áudio registrados, fechamento/relançamento/parada e logs. Uma decisão go deve registrar limitações e a composição efetivamente demonstrada.

Se somente Waydroid como está for demonstrado, apresentar ao usuário a mudança de B para A e solicitar uma nova decisão de arquitetura. Não implementar essa mudança implicitamente. A validação do app Roblox oficial, sem modificações, é uma etapa posterior independente.

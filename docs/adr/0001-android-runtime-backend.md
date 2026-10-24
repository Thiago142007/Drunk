# ADR 0001 — Backend Android: no-go provisório

**Data:** 2026-10-24 (adendo documental sobre redroid e políticas Roblox em 2026-10-24). **Decisão:** **NO-GO provisório para iniciar o runtime/produto. Backend de produto selecionado: nenhum.** O usuário autorizou um protótipo limitado com Android externo; redroid é candidato a esse spike em x86-64, não uma decisão de arquitetura do produto.

**Bloqueadores visíveis:** não há runtime de produto nem protótipo redroid demonstrado, inventário completo de licenças ou lançamento manual de app Android não Roblox. Gráficos, input, áudio e encerramento permanecem sem medição. O usuário autorizou carregar `binder_linux` e usar temporariamente um container Docker `--privileged`; a execução aguarda autenticação `sudo` interativa no terminal aberto. Nenhum módulo foi carregado nem container iniciado. A conclusão é falta de evidência suficiente, não impossibilidade comprovada.

## Contexto

A [especificação](../superpowers/specs/2026-10-24-drunk-design.md), seções 5–7, escolhe runtime Linux próprio em Rust com componentes Android externos (B) e impõe prova técnica antes do produto. A [avaliação](../runtime-evaluation.md) contém fontes, comandos da inspeção local, comparação e protocolo futuro.

O [README público do Sober](https://github.com/vinegarhq/sober) identifica um issue tracker; o [site](https://sober.vinegarhq.org/) informa que o projeto é fechado; a [FAQ](https://vinegarhq.org/Sober/FAQ/index.html) descreve um runtime para o cliente Android. Essas fontes não fornecem um contrato de integração nem permitem afirmar quais componentes internos ele usa.

## Alternativas e decisão

| Alternativa | Decisão e fundamento |
| --- | --- |
| Adaptador próprio com componentes AOSP | Preserva conceitualmente B. A [arquitetura AOSP](https://source.android.com/docs/core/architecture) e a [ABI x86_64](https://developer.android.com/ndk/guides/abis) estão documentadas, mas a integração mínima Linux não foi demonstrada. Licenças devem ser verificadas por componente; [AOSP não usa uma única licença](https://source.android.com/docs/setup/contribute/licenses). Não selecionar agora. |
| Waydroid como backend externo completo | [Waydroid documenta Android completo em container com namespaces](https://docs.waydro.id/). **Avaliação do projeto:** um supervisor Rust que apenas usa seu CLI corresponde à opção A, não comprova B. Manter como referência; exigir nova decisão do usuário para adotá-lo. |
| redroid como Android externo para spike | O [README oficial redroid](https://github.com/remote-android/redroid-doc) documenta suporte `amd64`/`arm64`, ADB TCP, módulos Binder/ashmem e exemplo Docker `--privileged`. O usuário autorizou protótipo Android externo, incluindo carregar `binder_linux` e usar temporariamente container privilegiado. **Decisão:** o spike está autorizado, mas aguarda autenticação `sudo` interativa no terminal aberto. Isso não seleciona redroid como produto nem prova B; nenhuma instalação ou execução foi feita. |
| Android completo em VM, representado pelo Android Emulator | O [emulador oficial](https://developer.android.com/studio/run/emulator) fornece um dispositivo Android virtual. **Avaliação do projeto:** é possível controle de laboratório, não prova do adaptador B. [Termos do SDK](https://developer.android.com/studio/terms) e licenças dos artefatos precisam ser resolvidos antes de distribuição. Não selecionar agora. |

A inspeção de leitura encontrou ADB e Sober instalado, mas não encontrou Waydroid/emuladores no PATH e nos caminhos adicionais registrados na avaliação. Nenhum app Android foi iniciado; não houve instalação, download de APK, alteração de serviço ou configuração. Portanto, o critério obrigatório de lançamento manual não foi satisfeito. A instalação do Sober não preenche esse critério para um runtime Drunk próprio. A avaliação documental do redroid o torna candidato para o spike Android externo; o usuário autorizou Binder e container privilegiado temporários, mas a autenticação `sudo` ainda não ocorreu e nenhum módulo/container foi iniciado.

## Primeiro marco publicado para revisão

Em 2026-10-24, o marco conserva a decisão e os bloqueadores acima. A [seção de diagnóstico local](../runtime-evaluation.md#31-diagnóstico-de-leitura-do-primeiro-marco) registra os resultados de leitura da tarefa 3 (`dd4b143`): Linux x86-64, kernel 7.0, SSE4.1/SSE4.2 presentes, dois nós DRI e arquivos de loaders Vulkan/EGL/GL presentes. São apenas indícios de capacidade da máquina; não demonstram APIs gráficas, permissões, funcionamento de drivers ou compatibilidade Android/Roblox. RAM disponível, compositor/versão e requisitos da composição futura continuam sem verificação.

O host foi inspecionado para o protocolo futuro; nenhum lançamento Android foi realizado. A composição B e um runtime para os samples não foram selecionados/preparados, os samples não foram preparados e não existe resultado de execução. O gate falhou nessa preparação, antes de qualquer medição de janela, input, áudio, gráficos ou parada.

O [resumo do marco](../runtime-evaluation.md#7-primeiro-marco-para-revisão) registra o gate. Um spike de redroid limitado a app Android de teste permitido está autorizado; sua execução neste host aguarda autenticação `sudo` interativa no terminal aberto. Até o momento, nenhum módulo foi carregado nem container iniciado. O spike não abre o gate para runtime Roblox nem para plano de produto. Adoção de redroid, Waydroid ou outro Android completo como produto exige uma decisão de arquitetura separada; nenhuma foi tomada.

## Consequências e riscos não resolvidos

- Pausar runtime de produto, pontes, instalador e qualquer alegação de compatibilidade. A investigação documental desta tarefa está concluída; o gate de execução continua reprovado.
- O custo de manter interfaces Android/Linux próprias permanece desconhecido. É uma **inferência de risco** decorrente da composição ainda indefinida.
- GPU/driver, privilégios, IPC, áudio e ciclo de vida não estão validados. A presença de Mesa ou `/dev/kvm` não substitui medição.
- Licenças de código aberto, imagens, SDK e aplicativos têm fronteiras distintas; nenhuma permissão de redistribuir Roblox ou serviços proprietários foi estabelecida.
- Um sample funcionar não garante Roblox, login, attestation ou experiências. Permanecem as restrições da especificação contra alteração do cliente e evasão de controles.
- redroid documenta licença Apache 2.0 para o projeto, GPLv2 para seus módulos de kernel e alerta para examinar licenças de módulos de terceiros. Uma tag/digest de imagem e inventário de componentes ainda precisam ser fixados e revisados antes de distribuição.
- Roblox informa que Linux não é suportado nos [requisitos de computador](https://en.help.roblox.com/hc/en-us/articles/203312800-Computer-Hardware-Operating-System-Requirements). A [documentação oficial de Android Remote Attestation](https://en.help.roblox.com/hc/en-us/articles/49648939984916-Android-Remote-Attestation) diz que emuladores/dispositivos virtuais são tratados como não confiáveis e que criadores podem exigir a verificação em certos jogos. Isso não prova o resultado de cada experiência; não se fará bypass, falsificação ou enfraquecimento de verificações.
- A autorização operacional para carregar `binder_linux` e usar temporariamente Docker `--privileged` foi concedida. Falta a autenticação `sudo` interativa no terminal aberto; até lá, nenhuma operação privilegiada nem inicialização redroid ocorreu.

## Reabertura

Reavaliar esta ADR depois do spike, após autenticação e execução, com versões/digest fixados, interface de controle, licenças e evidência do [protocolo manual](../runtime-evaluation.md#5-protocolo-manual-futuro--não-executado): app de teste permitido em janela, gráficos/input/áudio registrados, fechamento/relançamento/parada e logs. Mesmo um spike bem-sucedido mantém **NO-GO de produto** até uma decisão explícita de arquitetura, manutenção/distribuição e licenças, e não demonstra compatibilidade com Roblox.

Se o único caminho demonstrado for um Android completo pronto (redroid, Waydroid ou outro), apresentar ao usuário a mudança de arquitetura e pedir decisão antes de adotá-lo como produto. A validação do app Roblox oficial, sem modificações, seria uma etapa posterior e separada, condicionada a um método permitido de obtenção/instalação. Nenhuma compatibilidade Roblox é alegada aqui.

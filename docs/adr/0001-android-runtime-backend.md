# ADR 0001 — Backend Android: no-go provisório

**Data:** 2026-10-24 (adendo documental sobre redroid e políticas Roblox em 2026-10-24). **Decisão:** **NO-GO provisório para iniciar o runtime/produto. Backend de produto selecionado: nenhum.** O usuário autorizou um protótipo limitado com Android externo; redroid é candidato a esse spike em x86-64, não uma decisão de arquitetura do produto.

**Bloqueadores visíveis:** não há runtime de produto nem inventário completo de licenças. Um spike redroid demonstrou boot Android 12, ADB, abertura de Settings e toque sintético; janela desktop, gráficos acelerados, áudio audível, instalação de APK externo e encerramento ainda não foram validados. O usuário autorizou e realizou o carregamento temporário de `binder_linux` e uso de Docker `--privileged`. A conclusão é falta de evidência suficiente para produto, não impossibilidade comprovada.

## Contexto

A [especificação](../superpowers/specs/2026-10-24-drunk-design.md), seções 5–7, escolhe runtime Linux próprio em Rust com componentes Android externos (B) e impõe prova técnica antes do produto. A [avaliação](../runtime-evaluation.md) contém fontes, comandos da inspeção local, comparação e protocolo futuro.

O [README público do Sober](https://github.com/vinegarhq/sober) identifica um issue tracker; o [site](https://sober.vinegarhq.org/) informa que o projeto é fechado; a [FAQ](https://vinegarhq.org/Sober/FAQ/index.html) descreve um runtime para o cliente Android. Essas fontes não fornecem um contrato de integração nem permitem afirmar quais componentes internos ele usa.

## Alternativas e decisão

| Alternativa | Decisão e fundamento |
| --- | --- |
| Adaptador próprio com componentes AOSP | Preserva conceitualmente B. A [arquitetura AOSP](https://source.android.com/docs/core/architecture) e a [ABI x86_64](https://developer.android.com/ndk/guides/abis) estão documentadas, mas a integração mínima Linux não foi demonstrada. Licenças devem ser verificadas por componente; [AOSP não usa uma única licença](https://source.android.com/docs/setup/contribute/licenses). Não selecionar agora. |
| Waydroid como backend externo completo | [Waydroid documenta Android completo em container com namespaces](https://docs.waydro.id/). **Avaliação do projeto:** um supervisor Rust que apenas usa seu CLI corresponde à opção A, não comprova B. Manter como referência; exigir nova decisão do usuário para adotá-lo. |
| redroid como Android externo para spike | O [README oficial redroid](https://github.com/remote-android/redroid-doc) documenta suporte `amd64`/`arm64`, ADB TCP, módulos Binder/ashmem e exemplo Docker `--privileged`. O usuário autorizou protótipo Android externo, incluindo carregar `binder_linux` e usar temporariamente container privilegiado. **Decisão:** redroid foi executado como spike e iniciou Android 12; isso não seleciona redroid como produto nem prova B. |
| Android completo em VM, representado pelo Android Emulator | O [emulador oficial](https://developer.android.com/studio/run/emulator) fornece um dispositivo Android virtual. **Avaliação do projeto:** é possível controle de laboratório, não prova do adaptador B. [Termos do SDK](https://developer.android.com/studio/terms) e licenças dos artefatos precisam ser resolvidos antes de distribuição. Não selecionar agora. |

A inspeção de leitura inicial encontrou ADB e Sober instalado, mas não encontrou Waydroid/emuladores no PATH e nos caminhos adicionais registrados na avaliação. Na etapa posterior autorizada, redroid iniciou Android 12, abriu Settings por ADB e aceitou toque sintético; não houve instalação de APK externo. A instalação do Sober não preenche os critérios de um runtime Drunk próprio.

## Primeiro marco publicado para revisão

Em 2026-10-24, o marco conserva a decisão e os bloqueadores acima. A [seção de diagnóstico local](../runtime-evaluation.md#31-diagnóstico-de-leitura-do-primeiro-marco) registra os resultados de leitura da tarefa 3 (`dd4b143`): Linux x86-64, kernel 7.0, SSE4.1/SSE4.2 presentes, dois nós DRI e arquivos de loaders Vulkan/EGL/GL presentes. São apenas indícios de capacidade da máquina; não demonstram APIs gráficas, permissões, funcionamento de drivers ou compatibilidade Android/Roblox. RAM disponível, compositor/versão e requisitos da composição futura continuam sem verificação.

O spike registrou boot Android 12, ADB, lançamento da atividade Settings e toque sintético por ADB. Não houve medição de janela desktop, aceleração gráfica, áudio audível, instalação de APK externo nem parada executada pela CLI. O gate de produto permanece reprovado, pois o spike não demonstra a composição B nem resolve manutenção e licenças.

O [resumo do marco](../runtime-evaluation.md#7-primeiro-marco-para-revisão) registra o gate. O spike redroid limitado foi executado após autorização do usuário, mas não abre o gate para runtime Roblox nem para plano de produto. Adoção de redroid, Waydroid ou outro Android completo como produto exige uma decisão de arquitetura separada; nenhuma foi tomada.

## Consequências e riscos não resolvidos

- Manter pausados o runtime de produto, as pontes finais, o instalador e qualquer alegação de compatibilidade. O spike externo é evidência parcial; o gate de produto continua reprovado.
- O custo de manter interfaces Android/Linux próprias permanece desconhecido. É uma **inferência de risco** decorrente da composição ainda indefinida.
- GPU/driver, privilégios, IPC, áudio e ciclo de vida não estão validados. A presença de Mesa ou `/dev/kvm` não substitui medição.
- Licenças de código aberto, imagens, SDK e aplicativos têm fronteiras distintas; nenhuma permissão de redistribuir Roblox ou serviços proprietários foi estabelecida.
- Um sample funcionar não garante Roblox, login, attestation ou experiências. Permanecem as restrições da especificação contra alteração do cliente e evasão de controles.
- redroid documenta licença Apache 2.0 para o projeto, GPLv2 para seus módulos de kernel e alerta para examinar licenças de módulos de terceiros. O spike fixou a imagem por digest; inventário de componentes e revisão de licenças ainda faltam antes de qualquer distribuição.
- Roblox informa que Linux não é suportado nos [requisitos de computador](https://en.help.roblox.com/hc/en-us/articles/203312800-Computer-Hardware-Operating-System-Requirements). A [documentação oficial de Android Remote Attestation](https://en.help.roblox.com/hc/en-us/articles/49648939984916-Android-Remote-Attestation) diz que emuladores/dispositivos virtuais são tratados como não confiáveis e que criadores podem exigir a verificação em certos jogos. Isso não prova o resultado de cada experiência; não se fará bypass, falsificação ou enfraquecimento de verificações.
- A autorização para carregar `binder_linux` e usar Docker `--privileged` temporário foi exercida no laboratório. Container, volume, imagem e montagem BinderFS são temporários; seguir a limpeza registrada em `../redroid-spike.md`.

## Reabertura

Reavaliar esta ADR com os itens que faltam no [protocolo manual](../runtime-evaluation.md#5-protocolo-manual-futuro--não-executado): janela nativa, gráficos/input/áudio registrados, instalação e lançamento de app de teste permitido, fechamento/relançamento/parada controlada e logs. Mesmo um spike bem-sucedido mantém **NO-GO de produto** até uma decisão explícita de arquitetura, manutenção/distribuição e licenças, e não demonstra compatibilidade com Roblox.

Se o único caminho demonstrado for um Android completo pronto (redroid, Waydroid ou outro), apresentar ao usuário a mudança de arquitetura e pedir decisão antes de adotá-lo como produto. A validação do app Roblox oficial, sem modificações, seria uma etapa posterior e separada, condicionada a um método permitido de obtenção/instalação. Nenhuma compatibilidade Roblox é alegada aqui.

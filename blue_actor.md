# WIP

Queste sono alcune considerazioni per limare/stabilizzare il pattern actor model
ed il suo utilizzo.

Sono commenti fatti su usi specifici, per questo sono work-in-progress, ma preferisco
cristallizzarli nel repo che attualmente contiene il toktor crate.

## Passare dall'observer pattern ad actor model.

Fondamentale, almeno credo, dal punto di vista del TDD è l'implementazione di dipendenze injected.
Per questo l'observer pattern utilizza naturalmente/implicitament dynamic dispatch come meccanismo.

Dynamic dispatch è basato sulla definizione di una interfaccia (interface, abstract class, o
altri nomi) che definisce il comportamento della classe target, tramite la lista dei metodi
e la loro firma.

A differenza di un template/generics, il dynamic dispatch non richiede che il destinatario
sia conosciuto al momento della compilazione.

Utilizzando il generics, invece, è possibile implementare qualcosa di più limitato rispetto
all'observer pattern, cioè un observer pattern selettivo, deciso al momento della compilazione,
che però è limitato ad un singolo subscriber, e singolo observer alla volta.

Actor model prende forma in un ambiente di esecuzione concorrente, dove ogni thread o routine
di esecuzione comunica con gli altri tramite passaggio di messaggi, o accesso esclusivo a risorse
condivise (NO!).

Infatti l'actor model depreca l'accesso esclusivo a risorse condivise, e utilizza canali per
il message passing.

Mi interessa capire come è possibile mappare le caratteristiche dei 2 patterns:

> Method List  <----------------->  MessagePassing

In particolare in Rust, dove gli enum sono strutturati.
Infatti un channel in rust ha associato un type, il tipo del messaggio che può essere trasmesso sopra quel
channel. Ed il type è tipicamente un enum:

```
enum {
 MethodnameFirst {
    data: MyStruct,
    respond_to: oneshot::Receiver<MethodReturnStructType>
 },
 MethodnameSecond {
    data: String,
    respond_to: oneshot::Receiver<bool>
 }
}
```

## Un Actor Model sequenziale

A questo punto vado a considerare la proposta di implementazione fatta da Alice Ryhl in https://ryhl.io/blog/actors-with-tokio/
Sarebbe piu' corretto effettivamente parlare di proposte al plurale, visto che sono piu' "indicazioni per una possibile implementazione".

L'implementazione piu' semplice e' quella sequenziale, cioe' run lancia handle() che tratta un messaggio alla volta
tra quelle ricevute nel mpsc channel.

Ovviamente questa cosa e' limitata, una proposta accennata e' quella di usare piu' channel da cui consumare i messaggi.

Oppure run() puo' essere definito come async, e all'interno di handle_msg() si fa tokio::spawn() per evitare di bloccare
ad ogni handling di un messaggio.

Tuttavia se c'e' una risorsa gestita ed owned dall'actor questo spawn porta a dover gestire la mutua esclusione,
mutua esclusione all'interno dello stesso actor.

Sicuramente la situazione e' meno ingarbugliata del dover gestire accessi da piu' parti di codice, ma ad ogni modo c'e'
bisogno dei soliti Arc e Mutex.

Il fatto che il codice sia vicino visivamente non vuol dire che lo siano i rispettivi path di esecuzione, e questi
possono essere ben ingarbugliati.

Come dice Alice Ryhl nell'ultima parte: "Beware of Cycles".

Attualmente ho solo immaginato architetture di esecuzione lineari: un ingresso, una elaborazione, una uscita.
Anzi, tipicamente sono proprio 3 passi, per questo motivo non ho dato molto peso a questa considerazione,
Ma ne va certo tenuto conto in situazioni piu' complesse.

## Notification Subscription

Ma come ho detto all'inizio mi interessa implementare l'observer pattern con l'actor model, e quindi realizzare
una subscription e notification. (Questo puo' essere la sorgente di cicli, effettivamente).

In quale maniera un subscriber puo' sottoscrivere piu' tipologie di notifiche? Ovvero, come l'actor implementa
il dispatching delle notifiche?

1. Il subscriber fornisce un canale di ascolto mpsc::Sender<PolimophicMessageType>, e la lista di Message Types
2. L'actor clone() il canale e lo usa per mandare notifiche di piu' tipi su quel canale.
3. Il subscriber si mette in ascolto su quel canale.

Questa e' una sorta di inversione di actor model control.

Chiaramente la gestione di piu' tipi di notifiche avverra' all'interno, ognuna, di un separato contesto di esecuzione:
tokio::spawn(), ancora.

## Esplosione di contesti di esecuzione

Il numero di contesti di esecuzione aperti puo' crescere molto in questa maniera.
Non lo ritengo un problema perche' non corrispondono 1-to-1 a veri e propri thread, ma piuttosto a
strutture dati trasparenti, che sono in attesa di ottenere una risorsa di calcolo per proseguire.

## Limiti sul numero di messaggi to-process

Ma c'e' una caratteristica invece che potrebbe essere controllata, per evitare inutili
conflitti nell'accesso a variabili atomic: il numero di richieste parallele gestite
dal canale principale dei comandi.

Questa cosa puo' essere implementata tramite un semaphore usato all'interno della fn run().
In tal caso l'handle creato da spawn dovrebbe essere ritornato dal handle_message() method
che dovrebbe decrementare il semaphore.
Ma questo rende le cose piu' complicate del dovuto, aggiungendo ancora altri contesti
di esecuzione, proprio quelli che vorrebbero essere limitati.

## Fault tollerancy

Quello che puo' essere interessante e' la gestione dei fallimenti: cosa succede se una risorsa
dalla quale dipende una subscription viene persa in modo inaspettato?

Riprendendo lo schema descritto sopra, il subscriber fornisce un canale mpsc::Sender dove l'actor
che funge da Observer comunica messaggi di tipo PolimophicMessageType.
Quindi sarebbe sufficiente che l'observer che gestisce la risorsa, al momento della situazione anomala
notifichi con un messaggio del tipo PolimophicMessageType::ResourceDied, ad esempio.
A questo punto il chiamante, observer, dovra' gestire questa situazione.

Tipicamente il chiamante vorra' ripetere la sottoscrizione, quindi naturalmente il codice
dovrebbe essere organizzato in 2 loop annidati, quello piu' esterno interviene solo nel caso anomalo:

```
let resource_x = self.resource_x.clone();
let list_of_notification_types = self.list_of_notification_types.clone();
// more clone here
tokio::spawn(async move {
   loop {
     let resource_x = resource_x.clone();
     let list_of_notification_types = list_of_notification_types.clone();
     tokio::spawn(async move {
       let mut from_observer = observer.subscribe(list_of_notification_types);
       loop {
           match from_agent.recv().await {
               Some(msg) => {
                   match msg {
                       PolimophicMessageType::ResourceDied => break;
                       // regular message handling here
                       ...
                   }
               }
           }
       };
     }).await
   }
})
```

Ok, ma questo sembra un "viaggio senza ritorno, e senza controllo".

Una strategia alternativa per gestire situazioni anomale, e' quella di demandare al chiamante
esterno la gestione dell'eccezione.

Il codice sopra andrebbe naturalmente inserito all'interno dell'actor che sottoscrive le
notifiche per le quali ha interesse. L'azione di "sottoscrivere" e' tipicamente
comandata dall'handler dell'actor sottoscrittore, e prima ancora da un controllore
del processo.

In questo senso il sottoscrittore non ha nessuna necessita' di mantenere l'ownership
dell'observer.

Ma qui interviene un altro attore, in senso lato questa volta, cioe' qualche procedura
che e' responsabile della prima sottoscrizione dell'interessato all'observer.

Qual e' dunque la responsabilita' di gestire il fallimento e cancellazione della
sottoscrizione dovuto ad un evento anomalo avvenuto nell'observer?

## Blue Actor: actor controller

Mi rifaccio ai colori usati da Edward De Bono in sei cappelli per pensare, e definisco
un capostazione, un BlueActor, che si occupa di controllare che tutto vada bene,
in particolare:

1. avvia tutti gli actor necessari instanziandoli
2. gestisce le situazioni anomale
3. si occupa della chiusura della applicazione in caso di anomalia non recuperabile

Per permettere di ricevere notifiche di situazioni anomale da parte di tutti gli actor,
il BlueActor apre un canale mpsc, blue_channel, in lettura. Ogni actor ha naturalmente
un channel blue_channel_tx in scrittura, quindi questo e' un parametro di inizializzazione
di ogni actor.

D'altra parte per lo shutdown, ogni actor dovrebbe mettere a disposizione un messaggio
specifico.

Per gestire una situazione di fallimento come quella sopra, il blue_actor dovrebbe

1. ricevere un BlueMessage::RecoverableResourceFailure da parte dell'observer
2. notificare il subscriber della perdita di un observer (il subscriber deve riconoscere in qualche maniera che e' qualcosa che lo interessa)
3. ripetere la procedura di subscription per il subscriber

Infatti tra i compiti del blue_actor c'e' quello di istruire i diversi actor riguardo il setup iniziale.

E' naturale far interpretare il ruolo del blue_actor al fn main(), inserendolo il suo lavoro
il loop che contiene un'attesa sul blue_channel.

Chiaramente un qualche actor dovra'/potra' intercettare un'interruzione come Ctrl-C, o un suspend Ctrl-Z
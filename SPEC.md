Poniżej znajduje się kompletny, produkcyjny dokument **`SPEC.md`**. Zawiera on formalne modele matematyczne, specyfikację protokolarną EVM/Solana oraz struktury implementacyjne dla języka Rust.

W dokumencie obowiązuje bezwzględna zasada: **zero komentarzy w blokach kodu**.

Skopiuj poniższą treść w całości i zapisz ją w pliku **`BLACKSKY/SPEC.md`**.

---

# SPEC.MD: ARCHITEKTURA TECHNICZNA I EKONOMETRYCZNA SYSTEMU BLACKSKY

Data specyfikacji: 27 października 2026
Wersja protokołu: 1.0.0-PROD
Środowisko docelowe: Rust 1.80+ (edycja 2021) / Tokio / reqwest

---

## 1. AXIOMATYKA I REGUŁY IMPLEMENTACJI W JĘZYKU RUST

### 1.1. Topologia Modułowa
System przyjmuje płaską strukturę modułów w katalogu `src/` bez zagnieżdżania podfolderów. Odpowiedzialność modułów jest ściśle separowana:
* `src/config.rs` – Deterministyczny rejestr statyczny (adresy, stałe sieciowe, parametry ARR i emisji).
* `src/models.rs` – Domenowe struktury danych, definicje błędów i relacje obiektowe.
* `src/rpc.rs` – Asynchroniczna warstwa transportowa I/O dla węzłów JSON-RPC (Polygon PoS oraz Solana).
* `src/storage.rs` – Deterministyczny silnik append-only dla pliku szeregów czasowych `data/snapshots.csv`.
* `src/math.rs` – Dyskretny silnik ekonometryczny, dynamika nieliniowego spalania i cena równowagi.
* `src/main.rs` – Punkt wejścia orkiestrujący pobranie stanu, zapis i wyliczenie wskaźników.

### 1.2. Determinizm Błędów i Odporność
1. Zabrania się wywołań `unwrap()`, `expect()` oraz `panic!()` w jakimkolwiek module produkcyjnym (`rpc.rs`, `storage.rs`, `math.rs`).
2. Wszystkie operacje wejścia/wyjścia oraz operacje na pamięci zwracają zdefiniowany typ `Result<T, BlackskyError>`.
3. Błędy sieciowe (timeout, utrata połączenia, HTTP 429/500) oraz błędy dekodowania heksadecymalnego podlegają weryfikacji i opakowaniu w enumerację domenową.

### 1.3. Ograniczenie Zasobów i Zarządzanie Współbieżnością
1. Dostęp do publicznych węzłów RPC podlega rygorowi współbieżności z użyciem `tokio::sync::Semaphore`.
2. Maksymalna liczba równoległych żądań HTTP do sieci Polygon wynosi 5.
3. Klient HTTP (`reqwest::Client`) musi być zainicjalizowany jednokrotnie jako singleton z pulą połączeń (Connection Pooling) i czasem oczekiwania (Timeout) ustalonym sztywno na 10 sekund.

### 1.4. Doktryna Przechowywania Danych (Compute-on-Read)
1. Warstwa trwała przechowuje wyłącznie surowy wektor stanu: trójkę `(Timestamp, Address, Balance)`.
2. Zakazuje się zapisywania do bazy lub pliku wartości pochodnych: delty dziennej, prędkości, przyspieszenia ani wskaźników względnych. Wszelkie operatory różnicowe są obliczane wyłącznie w fazie analitycznej na podstawie surowego wektora stanu.

---

## 2. SPECYFIKACJA PROTOKOŁU I ŚRODOWISKA ON-CHAIN

### 2.1. Sieć Polygon PoS (Warstwa Pierwotna Tokena)
* Kontrakt tokena GEODNET (ERC-20): `0xac0f66379a6d7801d7726d5a943356a172549adb`
* Precyzja jednostki bazowej (Decimals): 18 miejsc po przecinku ($10^{18}$ wei = 1.0 GEOD).
* Kodowanie metody `balanceOf(address)`:
  * Selektor funkcji keccak256: `0x70a08231`
  * Format argumentu: adres Ethereum (20 bajtów) wyrównany do prawej w polu 32-bajtowym (dopełniony z lewej 24 zerami, łącznie 64 znaki hex bez prefiksu `0x`).
  * Struktura payloadu zapytania:
```json
{
  "jsonrpc": "2.0",
  "method": "eth_call",
  "params": [
    {
      "to": "0xac0f66379a6d7801d7726d5a943356a172549adb",
      "data": "0x70a08231000000000000000000000000[ADRES_20_BAJTOW]"
    },
    "latest"
  ],
  "id": 1
}
```
* Kodowanie metody `totalSupply()`:
  * Selektor funkcji keccak256: `0x18160ddd`
  * Format danych: `0x18160ddd` bez dodatkowych parametrów.
* Dekodowanie odpowiedzi liczbowej:
  * Pole `result` zawiera ciąg heksadecymalny liczby całkowitej bez znaku $U \in [0, 2^{256}-1]$.
  * Wartość skalarna tokena $B \in \mathbb{R}$ wyznaczana jest jako:
    $$B = \frac{U}{10^{18}}$$

### 2.2. Sieć Solana (Warstwa Obiegu CEX/DEX)
* Kontrakt tokena GEODNET (SPL Token): `7JA5eZdCzztSfQbJvS8aVVxMFfd81Rs9VvwnocV1mKHu`
* Precyzja jednostki bazowej (Decimals): 9 miejsc po przecinku ($10^9$ lamports = 1.0 GEOD).
* Metoda odpytywania węzła JSON-RPC: `getTokenAccountBalance` lub `getTokenAccountsByOwner` z filtrem mintu SPL.

### 2.3. Most Międzyłańcuchowy Wormhole NTT
* Polygon Custodian V1: `0x2006b44684b2a579466fc04fabc5a535946bc7ab`
* Polygon Transceiver V3: `0x6762157b73941e36cEd0AEf54614DdE545d0F990`
* Zasada zachowania masy: tokeny zablokowane w kontrakcie Custodian na Polygonie odpowiadają dokładnie wyemitowanej podaży SPL na sieci Solana.

### 2.4. Węzły Protokolarne i Wykonawcze
* Martwy adres spalania (Polygon Dead Address): `0x000000000000000000000000000000000000dEaD`
* Portfel skupu rynkowego (Buyback Executor): `0xc327C048d75398Da9DB5254679bb84a4a9e42010`

---

## 3. TRZY GŁÓWNE CELE SYSTEMU BLACKSKY (FAZA 1)

### CEL 1: Bezpośrednia Ingestia i Indeksacja On-Chain (Raw State Acquisition)
Zbudowanie autonomicznego mechanizmu odpytującego węzeł JSON-RPC sieci Polygon o stan kont 37 zdefiniowanych jednostek protokolarnych bez użycia zewnętrznych agregatorów danych.

1. Pobranie stanu całkowitego $S_{\text{total}}(t)$ oraz sald poszczególnych kont $B_i(t)$ dla $i \in \{1, \dots, 37\}$.
2. Konwersja surowych wielkich liczb całkowitych (256-bitowych) na wartości zmiennoprzecinkowe podwójnej precyzji `f64`.
3. Zapis wektora pomiarowego do pliku `data/snapshots.csv`.
4. Gwarancja idempotentności: program weryfikuje istnienie wpisów dla daty $t_{\text{current}}$ (UTC). W przypadku kolizji daty wykonanie zapisu zostaje pominięte bez powielania wierszy.

### CEL 2: Ekonometryczna Dekompozycja Podaży i Pomiar Wypływu VC
Podział całkowitej podaży Genesis $S_0 = 1\,000\,000\,000.0\text{ GEOD}$ na trzy zbiory rozłączne w dowolnej chwili czasu $t$:

$$S_0 = S_{\text{frozen}}(t) + S_{\text{burned}}(t) + S_{\text{overhang}}(t)$$

Gdzie:
1. **$S_{\text{frozen}}(t)$ (Zamrożona rezerwa protokolarna):**
   $$S_{\text{frozen}}(t) = B_{\text{MiningReserve}}(t) + B_{\text{TeamMasterVault}}(t) + B_{\text{PublicSale}}(t)$$
   Są to tokeny podlegające algorytmicznemu uwalnianiu długoterminowemu (halving) lub zamrożone w kontraktach pierwotnych.
2. **$S_{\text{burned}}(t)$ (Podaż trwale usunięta):**
   $$S_{\text{burned}}(t) = B_{\text{DeadAddress}}(t)$$
3. **$S_{\text{overhang}}(t)$ (Płynny nawis podażowy):**
   Suma sald portfeli powierniczych VC, portfeli dystrybucyjnych, rezerw giełd CEX, pul DEX oraz kontraktu mostu Wormhole:
   $$S_{\text{overhang}}(t) = \sum_{k \in \mathcal{K}_{\text{overhang}}} B_k(t)$$

Dla każdego portfela instytucjonalnego $k$ program definiuje dyskretny operator prędkości wypływu $v_k(t)$ dla kroku czasowego $\Delta t = 1\text{ doba}$:
$$v_k(t) = \frac{B_k(t) - B_k(t - \Delta t)}{\Delta t}$$

Rzeczywista dzienna presja podażowa generowana przez inwestorów instytucjonalnych wynosi:
$$\Phi_{\text{VC}}(t) = \sum_{k \in \mathcal{K}_{\text{VC}}} \max\left(0, -v_k(t)\right)$$

### CEL 3: Nieliniowy Model Dynamicznej Równowagi Deflacyjnej

#### 3.1. Nieliniowa Funkcja Spalania
Protokół przeznacza frakcję $\alpha = 0.80$ rocznego przychodu sieci $R_{\text{ARR}}(t) = 10\,640\,000.0\text{ USD}$ na rynkowy skup tokenów. Dzienny budżet skupu wynosi:
$$\Phi_{\text{USD}} = \frac{\alpha \cdot R_{\text{ARR}}(t)}{365}$$

Prędkość spalania tokenów $B(t, P)$ w jednostkach GEOD/dobę jest funkcją hiperboliczną względem rynkowej ceny tokena $P(t) \in \mathbb{R}^+$:
$$B(t, P) = \frac{\Phi_{\text{USD}}}{P(t)} = \frac{0.80 \cdot R_{\text{ARR}}(t)}{365 \cdot P(t)}$$

Własności analityczne funkcji spalania:
$$\lim_{P \to 0^+} B(t, P) = +\infty, \quad \lim_{P \to +\infty} B(t, P) = 0, \quad \frac{\partial B}{\partial P} = -\frac{\Phi_{\text{USD}}}{P^2} < 0$$
Wzrost ceny tokena asymptotycznie redukuje wolumen spalanych jednostek GEOD przy stałym przychodzie USD.

#### 3.2. Równanie Różniczkowe Zmiany Podaży Płynnej
Całkowity dzienny dopływ nowej podaży $\Phi_{\text{in}}(t)$ składa się z emisji dla stacji bazowych $E_{\text{miners}}(t)$ oraz zidentyfikowanego wypływu z portfeli VC $\Phi_{\text{VC}}(t)$:
$$\Phi_{\text{in}}(t) = E_{\text{miners}}(t) + \Phi_{\text{VC}}(t)$$
Gdzie $E_{\text{miners}}(t) = N_{\text{stations}}(t) \cdot r_{\text{station}} \cdot \bar{\eta}$, przy czym po halvingu z lipca 2026 stawka bazowa wynosi $r_{\text{station}} = 6.0\text{ GEOD/dzień}$.

Równanie różniczkowe ewolucji podaży rynkowej:
$$\frac{dS_{\text{circ}}}{dt} = \Phi_{\text{in}}(t) - B(t, P) - \Delta S_{\text{staked}}(t)$$

#### 3.3. Dynamiczna Cena Równowagi Deflacyjnej $P^*(t)$
Cena równowagi to wartość $P^*(t)$, przy której bilans przepływów płynnych wynosi zero ($\frac{dS_{\text{circ}}}{dt} = 0$, przy $\Delta S_{\text{staked}} = 0$):
$$\Phi_{\text{in}}(t) - \frac{\Phi_{\text{USD}}}{P^*(t)} = 0 \implies P^*(t) = \frac{0.80 \cdot R_{\text{ARR}}(t)}{365 \cdot \left[E_{\text{miners}}(t) + \Phi_{\text{VC}}(t)\right]}$$

Kryterium stanu sieci:
* $P(t) < P^*(t) \implies \frac{dS_{\text{circ}}}{dt} < 0$ (Sieć w reżimie deflacyjnym – popyt z buybacku przewyższa dopływ tokenów).
* $P(t) > P^*(t) \implies \frac{dS_{\text{circ}}}{dt} > 0$ (Sieć w reżimie inflacyjnym – spadek wolumenu skupu przy wyższej cenie nie równoważy emisji i zrzutów VC).

---

## 4. STRUKTURA DANYCH I FORMAT PLIKU SNAPSHOTS.CSV

### 4.1. Schemat Pliku `data/snapshots.csv`
Plik zorganizowany jest jako czysty format tekstowy CSV (RFC 4180), kodowany w UTF-8, bez powtarzania nagłówków kolumn w trakcie dopisywania.

Definicja kolumn:
1. `date` – Format daty ISO-8601: `YYYY-MM-DD` (UTC).
2. `address` – Ciąg znaków o stałej długości reprezentujący adres konta (format `0x` + 40 znaków hex dla Polygonu lub format Base58 dla Solany).
3. `balance_geod` – Liczba zmiennoprzecinkowa podwójnej precyzji `f64`, formatowana z dokładnością do 4 miejsc po przecinku (np. `40000010.0000`).

Przykładowy wiersz:
```text
2026-10-27,0x47c8ebecfa390a59b0e127c9178e936a4ada907d,40000010.0000
```

### 4.2. Struktury Domenowe w Języku Rust (`src/models.rs`)

```rust
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    Polygon,
    Solana,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCategory {
    ProtocolReserve,
    MiningDistribution,
    VCAllocationVault,
    TeamAllocationVault,
    EcosystemVault,
    VendorMarketingVault,
    PublicSaleVault,
    VCSubWallet,
    WhaleSmartMoney,
    MinerWhale,
    CEXWallet,
    DEXPool,
    BridgeCustodian,
    BuybackExecutor,
    StakingContract,
    BurnAddress,
}

#[derive(Debug, Clone)]
pub struct EntityMetadata {
    pub address: &'static str,
    pub chain: Chain,
    pub label: &'static str,
    pub category: EntityCategory,
    pub is_liquid_overhang: bool,
}

#[derive(Debug, Clone)]
pub struct BalanceSnapshot {
    pub date: String,
    pub address: String,
    pub balance_geod: f64,
}

#[derive(Debug, Clone)]
pub struct SupplyDecomposition {
    pub total_supply: f64,
    pub frozen_reserve: f64,
    pub burned_total: f64,
    pub liquid_overhang: f64,
    pub dynamic_equilibrium_price: f64,
}

#[derive(Debug)]
pub enum BlackskyError {
    NetworkError(String),
    RpcResponseError(String),
    HexDecodeError(String),
    StorageError(String),
    ConfigError(String),
}

impl fmt::Display for BlackskyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlackskyError::NetworkError(msg) => write!(f, "Network fault: {}", msg),
            BlackskyError::RpcResponseError(msg) => write!(f, "RPC fault: {}", msg),
            BlackskyError::HexDecodeError(msg) => write!(f, "Hex decode fault: {}", msg),
            BlackskyError::StorageError(msg) => write!(f, "Storage fault: {}", msg),
            BlackskyError::ConfigError(msg) => write!(f, "Config fault: {}", msg),
        }
    }
}

impl std::error::Error for BlackskyError {}
```

---

## 5. ZASADY WSPÓŁPRACY DEEPSEEK + AIDER + KOMPILATOR

1. **Weryfikacja Kompilatora:** Każda modyfikacja pliku źródłowego musi zostać natychmiast zweryfikowana przez `cargo check`. Wszelkie błędy typowania, naruszenia zasad pożyczania (Borrow Checker) oraz błędy obsługi `Result` są traktowane jako błędy krytyczne wymagające samoczynnej korekty w kolejnym kroku.
2. **Zakaz Komentarzy:** W generowanym kodzie nie może znaleźć się żaden komentarz składniowy (`//`, `/* */`, `#`). Kod musi tłumaczyć się wyłącznie poprzez ścisłe nazewnictwo zmiennych i funkcji oraz właściwe dopasowanie typów.
3. **Izolacja Zakresu:** Modyfikacji podlegają wyłącznie pliki wskazane w poleceniu. Niedozwolone jest wprowadzanie refaktoryzacji ani zmian sygnatur w plikach sąsiadujących, chyba że wynika to z konieczności naprawy błędu kompilacji zgłoszonego przez kompilator.
4. **Odporność Sieciowa:** Implementacja modułu `rpc.rs` musi radzić sobie z odpowiedziami węzłów RPC zawierającymi limity zapytań (HTTP 429) poprzez ponowienie próby (Backoff) lub bezpieczne opakowanie błędu w typ `BlackskyError::NetworkError`.
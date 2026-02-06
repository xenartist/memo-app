---
name: openclaw-x1-memo-protocol
description: Interact with the MEMO Protocol on X1 blockchain via free public JSON-RPC. Covers the full MEMO ecosystem including token minting, burning, transfers, user profiles, chat groups, projects, blogs, forums, and xDEX trading. Use when querying on-chain data (balances, token holders, burn stats, profiles, DEX pools) or building transactions against X1 mainnet/testnet.
---

# OpenClaw X1 MEMO Protocol Skill

Direct interaction with the MEMO Protocol on X1 blockchain via Solana-compatible JSON-RPC. No API keys needed.

## RPC Endpoint

```
Mainnet: https://rpc.mainnet.x1.xyz
Testnet: https://rpc.testnet.x1.xyz
```

All requests: `POST`, `Content-Type: application/json`, JSON-RPC 2.0 format.

## Request Template

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "<METHOD>",
  "params": [<PARAMS>]
}'
```

---

## Mainnet Program IDs & Token Addresses

```
Mint Program:       8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX
Burn Program:       2sb3gz5Cmr2g1ia5si2rmCZqPACxgaZXEmiS5k6Htcvh
Chat Program:       Hni4qE8GGW5uwBWzUEkpPBDRwXvKCWhM96teieAReRyd
Profile Program:    2BY8vPpQRFFwAqK3HqU5qL3qsGMH3VnX9Gv9bud3vzH8
Project Program:    6Vavot6ybhWBG3rjNXnLfNRPVTz7Garf6E4EZk3byp3a
Blog Program:       3EKdp88FgyPC41bxRDzFAtCDUMV2g9SVt5UiytE8wdzM
Forum Program:      6gzhG5BveTkJfTi466toX4qmN3BtU9qp1Grnk61GvmXD
xDEX Program:       sEsYH97wqmfnkzHedjNcw3zyJdPvUmsa9AixhS4b4fN

MEMO Token Mint:    memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick   (Token-2022, 6 decimals)
Token-2022 Program: TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
SPL Token Program:  TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
ATA Program:        ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
Memo Program (SPL): MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr

Native XNT:         So11111111111111111111111111111111111111111    (9 decimals)
Wrapped XNT (WXNT): So11111111111111111111111111111111111111112
USDC.X:             B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq
```

### MEMO Supply Tiers (Mint Reward Schedule)

| Supply Range | Reward per Mint |
|---|---|
| 0 - 100M | 1.0 MEMO |
| 100M - 1B | 0.1 MEMO |
| 1B - 10B | 0.01 MEMO |
| 10B - 100B | 0.001 MEMO |
| 100B - 1T | 0.0001 MEMO |
| 1T+ | 0.000001 MEMO |

---

## Read Operations (No Signing Required)

### 1. Get XNT Balance

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getBalance",
  "params": ["<PUBKEY>"]
}'
```

Response: `result.value` is balance in lamports. Divide by `1_000_000_000` for XNT.

### 2. Get MEMO Token Balance

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getTokenAccountsByOwner",
  "params": [
    "<OWNER_PUBKEY>",
    {"mint": "memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick"},
    {"encoding": "jsonParsed"}
  ]
}'
```

Response: `result.value[0].account.data.parsed.info.tokenAmount.uiAmount` is the human-readable balance.

### 3. Get MEMO Token Supply

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getTokenSupply",
  "params": ["memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick", {"commitment": "confirmed"}]
}'
```

Response: `result.value.amount` (raw), `result.value.uiAmount` (human-readable, 6 decimals).

### 4. Get Account Info

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getAccountInfo",
  "params": ["<PUBKEY>", {"encoding": "base64"}]
}'
```

Use `"encoding": "jsonParsed"` for token accounts to get parsed data automatically.

### 5. Get Transaction Details

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getTransaction",
  "params": ["<TX_SIGNATURE>", {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0}]
}'
```

### 6. Get Transaction History

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getSignaturesForAddress",
  "params": ["<ADDRESS>", {"limit": 20}]
}'
```

Options: `limit`, `before` (signature), `until` (signature), `commitment`.

### 7. Get Top MEMO Token Holders

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getProgramAccounts",
  "params": [
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
    {
      "encoding": "jsonParsed",
      "filters": [
        {"memcmp": {"offset": 0, "bytes": "memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick"}}
      ]
    }
  ]
}'
```

Each result: `account.data.parsed.info.owner` = holder address, `account.data.parsed.info.tokenAmount.uiAmount` = balance. Sort client-side by balance descending.

### 8. Get Top Burners

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getProgramAccounts",
  "params": [
    "2sb3gz5Cmr2g1ia5si2rmCZqPACxgaZXEmiS5k6Htcvh",
    {
      "encoding": "base64",
      "filters": [{"dataSize": 65}]
    }
  ]
}'
```

Parse each `UserGlobalBurnStats` account (65 bytes):

```
Offset  Size  Field
0       8     Discriminator (skip)
8       32    User pubkey
40      8     total_burned (u64 LE, divide by 1_000_000 for tokens)
48      8     burn_count (u64 LE)
56      8     last_burn_time (i64 LE, Unix timestamp)
64      1     bump
```

### 9. Get User Profile

Derive Profile PDA: `findProgramAddress([b"profile", user_pubkey_bytes], profile_program_id)`.

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getAccountInfo",
  "params": ["<PROFILE_PDA_ADDRESS>", {"encoding": "base64"}]
}'
```

Parse profile data:

```
Offset  Size      Field
0       8         Discriminator (skip)
8       32        User pubkey
40      4+N       Username (Borsh string: 4-byte LE length + UTF-8 bytes)
?       4+N       Image (Borsh string: hex-encoded avatar data)
?       8         created_at (i64 LE, Unix timestamp)
?       8         last_updated (i64 LE, Unix timestamp)
?       1+[4+N]   about_me (Borsh Option<String>: 0=None, 1+string=Some)
?       1         bump
```

### 10. Get RPC Version / Health Check

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getVersion",
  "params": []
}'
```

### 11. Get Latest Blockhash

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getLatestBlockhash",
  "params": [{"commitment": "confirmed", "minContextSlot": 0}]
}'
```

Response: `result.value.blockhash`.

### 12. Simulate Transaction

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "simulateTransaction",
  "params": [
    "<BASE64_ENCODED_TX>",
    {"encoding": "base64", "commitment": "confirmed", "replaceRecentBlockhash": true, "sigVerify": false}
  ]
}'
```

Response: `result.value.unitsConsumed` = compute units used, `result.value.err` = error (null if success).

### 13. Get Multiple Accounts (Batch)

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getMultipleAccounts",
  "params": [["<PUBKEY1>", "<PUBKEY2>"], {"encoding": "base64"}]
}'
```

---

## xDEX (DEX) Operations

### Get All Pools

```bash
curl -s https://rpc.mainnet.x1.xyz -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1,
  "method": "getProgramAccounts",
  "params": ["sEsYH97wqmfnkzHedjNcw3zyJdPvUmsa9AixhS4b4fN", {"encoding": "base64"}]
}'
```

Parse pool state (filter accounts with data length >= 400 bytes):

```
Offset  Size  Field
0       8     Discriminator
8       32    AMM Config
40      32    Pool Creator
72      32    Token 0 Vault
104     32    Token 1 Vault
136     32    LP Mint
168     32    Token 0 Mint
200     32    Token 1 Mint
232     32    Token 0 Program
264     32    Token 1 Program
296     32    Observation Key
328     1     Auth Bump
329     1     Status (0=all enabled, 1=deposit disabled, 2=withdraw disabled, 4=swap disabled)
330     1     LP Mint Decimals
331     1     Mint 0 Decimals
332     1     Mint 1 Decimals
333     8     LP Supply (u64 LE)
```

### Get Pool Price

1. Parse pool to get `token_0_vault` and `token_1_vault` addresses
2. Query each vault with `getAccountInfo` (`jsonParsed` encoding)
3. Extract `data.parsed.info.tokenAmount.amount` from each vault
4. `price = reserve_0 / reserve_1` (adjust for decimals)
5. If one token is USDC.X (`B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq`), you can derive USD prices

### Well-Known Tokens on xDEX

| Mint | Symbol | Name |
|---|---|---|
| `So11111111111111111111111111111111111111111` | XNT | X1 Native Token |
| `So11111111111111111111111111111111111111112` | WXNT | Wrapped XNT |
| `B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq` | USDC.X | USDC on X1 |
| `memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick` | MEMO | Memo Token |
| `pXNTyoqQsskHdZ7Q1rnP25FEyHHjissbs7n6RRN2nP5` | pXNT | Pooled XNT |
| `XBLKLmxhADMVX3DsdwymvHyYbBBfKa5eKhtpiQ2kj7T` | XBLK | XenBlocks |

---

## Account Data Parsing Guide

### Borsh Encoding (used by all X1 programs)

- **u8/u16/u32/u64/i64**: Little-endian bytes
- **String**: 4-byte LE length prefix + UTF-8 bytes
- **Vec<T>**: 4-byte LE length prefix + N items
- **Option<T>**: 1 byte flag (0=None, 1=Some) + optional T
- **Pubkey**: 32 bytes (Base58 when displayed)
- **Discriminator**: First 8 bytes of account data (Anchor uses SHA256 of `"account:<StructName>"`)

### Instruction Discriminator (Anchor convention)

Compute as: first 8 bytes of `SHA256("global:<instruction_name>")`.

Examples:
- `process_mint` → SHA256("global:process_mint")[..8]
- `create_profile` → SHA256("global:create_profile")[..8]
- `initialize_user_global_burn_stats` → SHA256("global:initialize_user_global_burn_stats")[..8]
- xDEX `swapBaseInput` discriminator: `[143, 190, 90, 218, 196, 30, 51, 222]`

### PDA Derivation Patterns

| Account Type | Seeds | Program |
|---|---|---|
| Mint Authority | `[b"mint_authority"]` | Mint Program |
| User Profile | `[b"profile", user_pubkey]` | Profile Program |
| User Burn Stats | `[b"user_global_burn_stats", user_pubkey]` | Burn Program |
| xDEX Authority | `[b"vault_and_lp_mint_auth_seed"]` | xDEX Program |
| Token ATA | `[owner, token_program, mint]` | ATA Program |

---

## Memo Serialization & Deserialization

All MEMO Protocol operations encode structured data into SPL Memo instructions. Understanding the encoding pipeline is critical for reading and writing on-chain data.

### Encoding Pipeline (Write)

There are two patterns depending on whether the operation involves burning:

**Pattern A: Chat messages (no burn)**

```
ChatMessageData struct
    │ Borsh serialize
    ▼
Binary bytes
    │ Base64 encode
    ▼
Base64 string (UTF-8 bytes → memo instruction data)
```

**Pattern B: Burn operations (profile, chat burn, group creation, etc.)**

```
Payload struct (e.g. ProfileCreationData)
    │ Borsh serialize
    ▼
Binary bytes (stored as BurnMemo.payload)
    │
BurnMemo { version: 1, burn_amount: N, payload: [...] }
    │ Borsh serialize
    ▼
Binary bytes
    │ Base64 encode
    ▼
Base64 string (UTF-8 bytes → memo instruction data)
```

### Decoding Pipeline (Read)

When reading memo data from a transaction:

```
Transaction memo field: "[length] base64_data"
    │ Strip "[length] " prefix (everything before first space)
    ▼
Base64 string
    │ Base64 decode
    ▼
Borsh binary bytes
    │ Borsh deserialize (try ChatMessageData or BurnMemo)
    ▼
Structured data
```

### Borsh Serialization Reference

All types are serialized with Borsh (Binary Object Representation Serializer for Hashing):

| Rust Type | Borsh Binary Layout | Example |
|---|---|---|
| `u8` | 1 byte | `0x01` → 1 |
| `u64` | 8 bytes, little-endian | `0x40420F0000000000` → 1_000_000 |
| `i64` | 8 bytes, little-endian, signed | same as u64 but signed |
| `String` | 4-byte LE length + UTF-8 bytes | `0x05000000` + `"hello"` → "hello" |
| `Vec<u8>` | 4-byte LE length + raw bytes | `0x03000000` + `0xAABBCC` |
| `Vec<String>` | 4-byte LE count + N strings | `0x02000000` + string1 + string2 |
| `Option<T>` | `0x00` (None) or `0x01` + T (Some) | `0x00` → None, `0x01` + data → Some(data) |
| `bool` | 1 byte | `0x00` → false, `0x01` → true |

### Data Structures

#### ChatMessageData (Pattern A — direct Borsh+Base64)

```
Field          Type            Borsh Bytes
version        u8              01
category       String          04000000 63686174                ("chat")
operation      String          0C000000 73656E645F6D657373616765 ("send_message")
group_id       u64             8 bytes LE
sender         String          4+N bytes (pubkey string)
message        String          4+N bytes (1-512 chars)
receiver       Option<String>  00 or 01+String
reply_to_sig   Option<String>  00 or 01+String
```

#### BurnMemo (Pattern B — outer wrapper)

```
Field          Type            Borsh Bytes
version        u8              01
burn_amount    u64             8 bytes LE (in lamports, ÷1_000_000 for tokens)
payload        Vec<u8>         4-byte LE length + raw bytes (inner Borsh-encoded struct)
```

#### ProfileCreationData (BurnMemo payload)

```
Field          Type            Values
version        u8              1
category       String          "profile"
operation      String          "create_profile" or "update_profile"
user_pubkey    String          Base58 pubkey string
username       String          1-32 chars
image          String          max 256 chars (hex-encoded avatar)
about_me       Option<String>  max 128 chars
```

#### ChatGroupBurnData (BurnMemo payload)

```
Field          Type            Values
version        u8              1
category       String          "chat"
operation      String          "burn_for_group"
group_id       u64             target group ID
burner         String          Base58 pubkey string
message        String          max 512 chars
```

#### ChatGroupCreationData (BurnMemo payload)

```
Field              Type            Values
version            u8              1
category           String          "chat"
operation          String          "create_group"
group_id           u64             expected group ID
name               String          1-64 chars
description        String          max 128 chars
image              String          max 256 chars
tags               Vec<String>     max 4 tags, each max 32 chars
min_memo_interval  Option<i64>     seconds (default 60)
```

### Example: Serializing a Chat Message

Suppose we want to encode a chat message: group_id=1, sender="ABC...XYZ", message="Hello!"

**Step 1: Borsh serialize ChatMessageData**

```
01                                          # version: u8 = 1
04000000 63686174                           # category: String = "chat" (len=4)
0C000000 73656E645F6D657373616765           # operation: String = "send_message" (len=12)
0100000000000000                            # group_id: u64 = 1
2C000000 414243...58595A                    # sender: String = "ABC...XYZ" (len=44, typical Base58 pubkey)
06000000 48656C6C6F21                       # message: String = "Hello!" (len=6)
00                                          # receiver: Option<String> = None
00                                          # reply_to_sig: Option<String> = None
```

**Step 2: Base64 encode the binary**

```python
import base64
memo_base64 = base64.b64encode(borsh_bytes).decode('utf-8')
# Result: "AQQAAABjaGF0DAAAAHN..."
```

**Step 3: Use as memo instruction data** (the Base64 string's UTF-8 bytes)

### Example: Serializing a Profile Burn

Create profile with burn_amount=420 MEMO, username="alice", image="ff00ff..."

**Step 1: Borsh serialize ProfileCreationData → payload bytes**

```
01                                          # version: u8 = 1
07000000 70726F66696C65                     # category: "profile" (len=7)
0E000000 6372656174655F70726F66696C65       # operation: "create_profile" (len=14)
2C000000 <pubkey_string_bytes>              # user_pubkey: Base58 string
05000000 616C696365                         # username: "alice" (len=5)
08000000 6666303066662E2E2E                 # image: "ff00ff..." (len=8)
00                                          # about_me: None
```

**Step 2: Wrap in BurnMemo and Borsh serialize**

```
01                                          # version: u8 = 1
00C2EB0B00000000                            # burn_amount: u64 = 420_000_000 (420 × 1_000_000)
<4-byte LE length> <payload_bytes>          # payload: Vec<u8>
```

**Step 3: Base64 encode → memo instruction data**

### Example: Decoding Memo from a Transaction (Python)

```python
import base64, struct

def decode_borsh_string(data, offset):
    """Decode a Borsh string: 4-byte LE length + UTF-8 bytes"""
    length = struct.unpack_from('<I', data, offset)[0]
    offset += 4
    value = data[offset:offset + length].decode('utf-8')
    return value, offset + length

def decode_chat_message(memo_field):
    """Decode a chat message from transaction memo field"""
    # Step 1: Strip "[length] " prefix from memo field
    if ' ' in memo_field:
        memo_b64 = memo_field.split(' ', 1)[1]
    else:
        memo_b64 = memo_field

    # Step 2: Base64 decode
    borsh_bytes = base64.b64decode(memo_b64)

    # Step 3: Borsh deserialize ChatMessageData
    offset = 0
    version = borsh_bytes[offset]; offset += 1
    category, offset = decode_borsh_string(borsh_bytes, offset)
    operation, offset = decode_borsh_string(borsh_bytes, offset)

    if category == "chat" and operation == "send_message":
        group_id = struct.unpack_from('<Q', borsh_bytes, offset)[0]; offset += 8
        sender, offset = decode_borsh_string(borsh_bytes, offset)
        message, offset = decode_borsh_string(borsh_bytes, offset)
        return {"type": "chat", "sender": sender, "message": message, "group_id": group_id}

    return None

def decode_burn_memo(memo_field):
    """Decode a burn memo from transaction memo field"""
    if ' ' in memo_field:
        memo_b64 = memo_field.split(' ', 1)[1]
    else:
        memo_b64 = memo_field

    borsh_bytes = base64.b64decode(memo_b64)

    # Outer BurnMemo structure
    offset = 0
    version = borsh_bytes[offset]; offset += 1
    burn_amount = struct.unpack_from('<Q', borsh_bytes, offset)[0]; offset += 8
    payload_len = struct.unpack_from('<I', borsh_bytes, offset)[0]; offset += 4
    payload = borsh_bytes[offset:offset + payload_len]

    # Try to parse payload
    p_offset = 0
    p_version = payload[p_offset]; p_offset += 1
    p_category, p_offset = decode_borsh_string(payload, p_offset)
    p_operation, p_offset = decode_borsh_string(payload, p_offset)

    if p_category == "profile":
        user_pubkey, p_offset = decode_borsh_string(payload, p_offset)
        username, p_offset = decode_borsh_string(payload, p_offset)
        image, p_offset = decode_borsh_string(payload, p_offset)
        return {
            "type": "profile", "operation": p_operation,
            "burn_tokens": burn_amount // 1_000_000,
            "user": user_pubkey, "username": username, "image": image
        }
    elif p_category == "chat" and p_operation == "burn_for_group":
        group_id = struct.unpack_from('<Q', payload, p_offset)[0]; p_offset += 8
        burner, p_offset = decode_borsh_string(payload, p_offset)
        message, p_offset = decode_borsh_string(payload, p_offset)
        return {
            "type": "chat_burn", "burn_tokens": burn_amount // 1_000_000,
            "group_id": group_id, "burner": burner, "message": message
        }

    return {"type": "unknown", "category": p_category, "operation": p_operation}
```

### Example: Encoding a Chat Message (Python)

```python
import base64, struct

def encode_borsh_string(s):
    """Encode a string as Borsh: 4-byte LE length + UTF-8 bytes"""
    encoded = s.encode('utf-8')
    return struct.pack('<I', len(encoded)) + encoded

def encode_borsh_option_string(value):
    """Encode Option<String>: 0x00 for None, 0x01 + string for Some"""
    if value is None:
        return b'\x00'
    return b'\x01' + encode_borsh_string(value)

def encode_chat_message(group_id, sender_pubkey, message, receiver=None, reply_to=None):
    """Encode a ChatMessageData as Base64 for memo instruction"""
    data = b''
    data += struct.pack('B', 1)                       # version: u8 = 1
    data += encode_borsh_string("chat")               # category
    data += encode_borsh_string("send_message")       # operation
    data += struct.pack('<Q', group_id)                # group_id: u64
    data += encode_borsh_string(sender_pubkey)         # sender
    data += encode_borsh_string(message)               # message
    data += encode_borsh_option_string(receiver)       # receiver: Option<String>
    data += encode_borsh_option_string(reply_to)       # reply_to_sig: Option<String>
    return base64.b64encode(data).decode('utf-8')

# Usage:
memo_b64 = encode_chat_message(1, "ABC...XYZ", "Hello world!")
# This string becomes the memo instruction data
```

### Memo Constraints
- Minimum memo length: 69 bytes (Base64 string length)
- Maximum memo length: 800 bytes (Base64 string length)
- Minimum burn for profile creation: 420 MEMO tokens
- Minimum burn for chat group creation: 42,069 MEMO tokens
- Minimum burn for group burn: 1 MEMO token
- Message max length: 512 characters

---

## Pixel Art Encoding (Profile & Chat Images)

The `image` field in profiles and chat groups stores a **1-bit pixel art** encoded as a compact ASCII string. Each pixel is either black (1) or white (0).

### Storage Format

The on-chain string uses one of two formats:

```
Normal:     "n:WxH:DATA"       e.g. "n:32x32:#####..."
Compressed: "c:WxH:BASE64"     e.g. "c:32x32:eJztwTEBAAAA..."
```

- `n` = normal (uncompressed safe-string)
- `c` = compressed (Deflate-compressed safe-string, then Base64-encoded)
- `WxH` = width x height (e.g. `32x32`, `64x64`, `96x96`)
- Compression is only used when it produces a shorter result

Legacy format (backward compatible): `"n:DATA"` or `"c:BASE64"` (without WxH, assumes 32x32 and auto-detects from string length).

### Supported Sizes

| Size | Pixels | Safe String Chars |
|---|---|---|
| 8x8 | 64 | 11 |
| 16x16 | 256 | 43 |
| 32x32 | 1024 | 171 |
| 64x64 | 4096 | 683 |
| 96x96 | 9216 | 1536 |
| 128x128 | 16384 | 2731 |

### Safe String Encoding Algorithm

Pixels are stored as a flat array in **row-major order** (left→right, top→bottom). Each pixel is 1 bit (true=black, false=white).

**Encoding (pixels → string):**

1. Read pixel bits sequentially, 6 bits at a time
2. Map each 6-bit value (0-63) to a safe ASCII character
3. If remaining bits < 6, left-shift and pad with zeros

**Character mapping** (`value` → ASCII):
```
ascii = 35 + value
if ascii >= 58: ascii += 1    (skip ':')
if ascii >= 92: ascii += 1    (skip '\')
```

This produces characters in range `#` (35) to `~` (126), avoiding `:`, `\`, and `"`.

**Decoding (string → pixels):**

1. For each character, reverse the mapping to get 6-bit value
2. Extract bits from MSB to LSB (bit 5 down to bit 0)
3. Each bit → one pixel (1=black, 0=white)

**Reverse character mapping** (`char` → value):
```
if char is ':' or '\' or '"': invalid
value = ascii - 35
if ascii > 92: value -= 1    (adjust for skipped '\')
if ascii > 58: value -= 1    (adjust for skipped ':')
```

### Compression (Optimal String)

When encoding for storage:

1. Generate the safe string from pixels
2. Deflate-compress the safe string bytes
3. Base64-encode the compressed bytes
4. If `"c:WxH:" + base64` is shorter than `"n:WxH:" + safe_string`, use compressed format

### Example: Decoding a Pixel Art Image (Python)

```python
import base64, zlib

def decode_pixel_char(c):
    """Reverse map a safe ASCII char to 6-bit value"""
    if c in (':', '\\', '"'):
        return None
    ascii_val = ord(c)
    if ascii_val < 35 or ascii_val > 126:
        return None
    value = ascii_val - 35
    if ascii_val > 92: value -= 1  # adjust for '\'
    if ascii_val > 58: value -= 1  # adjust for ':'
    return value if value < 64 else None

def decode_pixel_art(image_string):
    """Decode pixel art from on-chain image string → 2D boolean grid"""
    # Parse format
    parts = image_string.split(':', 2)

    if len(parts) == 3:
        fmt, size_str, data = parts
        w, h = map(int, size_str.split('x'))
    elif len(parts) == 2:
        fmt, data = parts
        w, h = 32, 32  # legacy default
    else:
        return None

    # Decompress if needed
    if fmt == 'c':
        compressed = base64.b64decode(data)
        safe_string = zlib.decompress(compressed, -15).decode('ascii')  # raw Deflate
    elif fmt == 'n':
        safe_string = data
    else:
        return None

    # Decode safe string to pixels
    pixels = []
    for c in safe_string:
        value = decode_pixel_char(c)
        if value is None:
            return None
        for i in range(5, -1, -1):  # bits 5 down to 0
            pixels.append(bool(value & (1 << i)))

    # Trim to exact pixel count and reshape to 2D grid
    pixels = pixels[:w * h]
    grid = [pixels[y * w:(y + 1) * w] for y in range(h)]
    return grid  # grid[row][col] = True (black) / False (white)

# Usage:
grid = decode_pixel_art("n:32x32:####$#$#$...")
# grid[0][0] = top-left pixel, True=black, False=white
```

### Example: Encoding a Pixel Art Image (Python)

```python
import base64, zlib

def encode_pixel_char(value):
    """Map 6-bit value (0-63) to safe ASCII char"""
    ascii_val = 35 + value
    if ascii_val >= 58: ascii_val += 1  # skip ':'
    if ascii_val >= 92: ascii_val += 1  # skip '\'
    return chr(ascii_val)

def encode_pixel_art(grid, width, height):
    """Encode 2D boolean grid → on-chain image string"""
    # Flatten grid to bit stream
    bits = [pixel for row in grid for pixel in row]

    # Encode 6 bits at a time
    safe_chars = []
    for i in range(0, len(bits), 6):
        chunk = bits[i:i+6]
        value = 0
        for bit in chunk:
            value = (value << 1) | int(bit)
        # Pad if chunk < 6 bits
        if len(chunk) < 6:
            value <<= (6 - len(chunk))
        safe_chars.append(encode_pixel_char(value))

    safe_string = ''.join(safe_chars)

    # Try compression
    compressed = zlib.compress(safe_string.encode('ascii'), 9)[2:-4]  # raw Deflate
    compressed_b64 = base64.b64encode(compressed).decode('ascii')

    normal_result = f"n:{width}x{height}:{safe_string}"
    compressed_result = f"c:{width}x{height}:{compressed_b64}"

    return compressed_result if len(compressed_result) < len(normal_result) else normal_result

# Usage: Create a 32x32 checkerboard
grid = [[(x + y) % 2 == 0 for x in range(32)] for y in range(32)]
image_string = encode_pixel_art(grid, 32, 32)
# Store this string in the 'image' field of ProfileCreationData
```

### Rendering Pixel Art

To display the decoded pixel art:
- Each `True` value = black pixel (filled)
- Each `False` value = white pixel (empty)
- Render as a square grid at the decoded `width x height`
- Common display: HTML canvas, SVG, or terminal block characters

---

## Write Operations (Transaction Building & Sending)

All write operations follow the same pattern:

1. **Build instructions** (memo instruction MUST be at index 0 if required by contract)
2. **Get latest blockhash** → `getLatestBlockhash`
3. **Simulate** with dummy compute budget (400k-1.4M CU) + `sigVerify: false, replaceRecentBlockhash: true`
4. **Extract** `unitsConsumed` from simulation
5. **Build final transaction** with real compute budget
6. **Sign** with user's keypair (Ed25519)
7. **Send** → `sendTransaction` with `{"encoding": "base64", "preflightCommitment": "confirmed", "maxRetries": 3}`

Transaction format: Bincode serialize → Base64 encode.

Use `solders` (Python) or `@solana/web3.js` (JS) to build and sign transactions.

### W1. Mint MEMO Token

Mint rewards user with MEMO tokens for writing a memo on-chain. Requires a memo instruction (69-800 bytes) at index 0.

**Discriminator**: `SHA256("global:process_mint")[..8]`

**Instruction Data**: discriminator only (8 bytes)

**Accounts** (in order):

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | yes | User's wallet (fee payer) |
| 1 | mint | no | yes | MEMO token mint (`memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick`) |
| 2 | mint_authority | no | no | PDA: `[b"mint_authority"]` from Mint Program |
| 3 | token_account | no | yes | User's MEMO ATA (create if not exists) |
| 4 | token_2022_program | no | no | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` |
| 5 | instructions_sysvar | no | no | `Sysvar1nstructions1111111111111111111111111` |

**Instructions order**: [SPL Memo, (Create ATA if needed), Mint Instruction, ComputeBudget]

**Memo content**: Plain text (69-800 bytes). The user writes anything they want — this is the "memo" that gets engraved on-chain.

### W2. Transfer Native XNT

**Instructions**: Use Solana `SystemProgram.transfer(from, to, lamports)`.

1 XNT = 1,000,000,000 lamports.

### W3. Transfer MEMO Token (SPL Token-2022)

**Instructions**: Use `spl-token-2022` `transfer_checked` instruction.

| Field | Value |
|---|---|
| Token Program | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` |
| Mint | `memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick` |
| Decimals | 6 |

If destination ATA doesn't exist, prepend a `CreateAssociatedTokenAccount` instruction.

### W4. Create User Profile

Burns MEMO tokens to create an on-chain profile with username and pixel art avatar.

**Discriminator**: `SHA256("global:create_profile")[..8]`

**Instruction Data**: discriminator (8 bytes) + burn_amount_units (u64 LE)

**Accounts** (in order):

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | yes | User's wallet |
| 1 | profile_pda | no | yes | PDA: `[b"profile", user_pubkey]` from Profile Program |
| 2 | memo_token_mint | no | yes | MEMO mint address |
| 3 | user_token_account | no | yes | User's MEMO ATA |
| 4 | user_burn_stats | no | yes | PDA: `[b"user_global_burn_stats", user_pubkey]` from Burn Program |
| 5 | token_2022_program | no | no | Token-2022 program |
| 6 | memo_burn_program | no | no | Burn Program ID |
| 7 | system_program | no | no | `11111111111111111111111111111111` |
| 8 | instructions_sysvar | no | no | Instructions sysvar |

**Memo (index 0)**: BurnMemo { version: 1, burn_amount, payload: ProfileCreationData }

**Minimum burn**: 420 MEMO tokens (burn_amount_units = 420 × 1,000,000)

### W5. Update User Profile

Same as create, but uses `SHA256("global:update_profile")[..8]` discriminator.

**Accounts** (in order):

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | yes | User's wallet |
| 1 | memo_token_mint | no | yes | MEMO mint address |
| 2 | user_token_account | no | yes | User's MEMO ATA |
| 3 | profile_pda | no | yes | Profile PDA |
| 4 | user_burn_stats | no | yes | User burn stats PDA |
| 5 | token_2022_program | no | no | Token-2022 program |
| 6 | instructions_sysvar | no | no | Instructions sysvar |
| 7 | memo_burn_program | no | no | Burn Program ID |

**Memo (index 0)**: BurnMemo { version: 1, burn_amount, payload: ProfileUpdateData }

### W6. Delete User Profile

**Discriminator**: `SHA256("global:delete_profile")[..8]`

**Instruction Data**: discriminator only (8 bytes)

**Accounts**:

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | yes | User's wallet |
| 1 | profile_pda | no | yes | PDA: `[b"profile", user_pubkey]` from Profile Program |

No memo required.

### W7. Send Chat Message

Sends a message to a chat group and earns MEMO mint rewards.

**Discriminator**: `SHA256("global:send_memo_to_group")[..8]`

**Instruction Data**: discriminator (8 bytes) + group_id (u64 LE)

**Accounts** (in order):

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | yes | User's wallet |
| 1 | chat_group_pda | no | yes | PDA: `[b"chat_group", group_id.to_le_bytes()]` from Chat Program |
| 2 | memo_token_mint | no | yes | MEMO mint address |
| 3 | mint_authority | no | no | PDA: `[b"mint_authority"]` from Mint Program |
| 4 | user_token_account | no | yes | User's MEMO ATA (create if needed) |
| 5 | token_2022_program | no | no | Token-2022 program |
| 6 | memo_mint_program | no | no | Mint Program ID |
| 7 | instructions_sysvar | no | no | Instructions sysvar |

**Memo (index 0)**: ChatMessageData (Borsh → Base64). Message max 512 chars.

### W8. Create Chat Group

Burns MEMO tokens to create a new chat group.

**Discriminator**: `SHA256("global:create_chat_group")[..8]`

**Instruction Data**: discriminator (8 bytes) + expected_group_id (u64 LE) + burn_amount (u64 LE)

Get `expected_group_id` by querying global counter PDA: `[b"global_counter"]` from Chat Program.

**Accounts** (in order):

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | yes | User's wallet |
| 1 | global_counter | no | yes | PDA: `[b"global_counter"]` from Chat Program |
| 2 | chat_group_pda | no | yes | PDA: `[b"chat_group", expected_group_id.to_le_bytes()]` from Chat Program |
| 3 | burn_leaderboard | no | yes | PDA: `[b"burn_leaderboard"]` from Chat Program |
| 4 | memo_token_mint | no | yes | MEMO mint address |
| 5 | user_token_account | no | yes | User's MEMO ATA |
| 6 | user_burn_stats | no | yes | PDA: `[b"user_global_burn_stats", user_pubkey]` from Burn Program |
| 7 | token_2022_program | no | no | Token-2022 program |
| 8 | memo_burn_program | no | no | Burn Program ID |
| 9 | system_program | no | no | System program |
| 10 | instructions_sysvar | no | no | Instructions sysvar |

**Memo (index 0)**: BurnMemo { version: 1, burn_amount, payload: ChatGroupCreationData }

**Minimum burn**: 42,069 MEMO tokens

### W9. Burn Tokens for Chat Group

Burns MEMO tokens for a group with an optional message.

**Discriminator**: `SHA256("global:burn_tokens_for_group")[..8]`

**Instruction Data**: discriminator (8 bytes) + group_id (u64 LE) + amount (u64 LE)

**Accounts** (in order):

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | yes | User's wallet |
| 1 | chat_group_pda | no | yes | Chat group PDA |
| 2 | burn_leaderboard | no | yes | Burn leaderboard PDA |
| 3 | memo_token_mint | no | yes | MEMO mint address |
| 4 | user_token_account | no | yes | User's MEMO ATA |
| 5 | user_burn_stats | no | yes | User burn stats PDA |
| 6 | token_2022_program | no | no | Token-2022 program |
| 7 | memo_burn_program | no | no | Burn Program ID |
| 8 | instructions_sysvar | no | no | Instructions sysvar |

**Memo (index 0)**: BurnMemo { version: 1, burn_amount, payload: ChatGroupBurnData }

**Minimum burn**: 1 MEMO token

### W10. Initialize Burn Stats (PREREQUISITE)

**IMPORTANT**: This must be executed ONCE per user BEFORE any burn operation (W4, W5, W8, W9, etc.). If the user's burn stats account does not exist, all burn transactions will fail. Query the stats PDA with `getAccountInfo` first — if `result.value` is `null`, call this instruction before proceeding with any burn.

**Discriminator**: `SHA256("global:initialize_user_global_burn_stats")[..8]`

**Instruction Data**: discriminator only (8 bytes)

**Accounts**:

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | yes | User's wallet |
| 1 | stats_pda | no | yes | PDA: `[b"user_global_burn_stats", user_pubkey]` from Burn Program |
| 2 | system_program | no | no | System program |

No memo required.

### W11. xDEX Swap (swapBaseInput)

Swap tokens on the xDEX DEX.

**Discriminator**: `[143, 190, 90, 218, 196, 30, 51, 222]` (fixed)

**Instruction Data**: discriminator (8 bytes) + amount_in (u64 LE) + minimum_amount_out (u64 LE)

**Accounts** (in order):

| # | Account | Signer | Writable | Description |
|---|---------|--------|----------|-------------|
| 0 | user | yes | no | User (signer) |
| 1 | authority | no | no | PDA: `[b"vault_and_lp_mint_auth_seed"]` from xDEX Program |
| 2 | amm_config | no | no | AMM config (from pool data offset 8) |
| 3 | pool_state | no | yes | Pool account address |
| 4 | input_token_account | no | yes | User's input ATA |
| 5 | output_token_account | no | yes | User's output ATA |
| 6 | input_vault | no | yes | Pool's input vault |
| 7 | output_vault | no | yes | Pool's output vault |
| 8 | input_token_program | no | no | Token program for input |
| 9 | output_token_program | no | no | Token program for output |
| 10 | input_mint | no | no | Input token mint |
| 11 | output_mint | no | no | Output token mint |
| 12 | observation_state | no | yes | Observation account (from pool data offset 296) |

Create input/output ATAs if they don't exist. For native XNT swaps, wrap to WXNT first (see W12).

### W12. Wrap Native XNT to WXNT

Wrap native XNT into SPL Token WXNT for DEX trading.

**Instructions** (3 steps):

1. **Create WXNT ATA** (if not exists): ATA program, mint = `So11111111111111111111111111111111111111112`, token program = `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` (standard SPL Token, NOT Token-2022)
2. **Transfer native XNT**: `SystemProgram.transfer(user, wxnt_ata, amount_lamports)`
3. **SyncNative**: Instruction to standard SPL Token program, account = wxnt_ata, data = `[17]`

### Transaction Building Example (Python with solders)

```python
from solders.keypair import Keypair
from solders.pubkey import Pubkey
from solders.instruction import Instruction, AccountMeta
from solders.transaction import Transaction
from solders.message import Message
from solders.hash import Hash
from solders.compute_budget import set_compute_unit_limit
import base64, struct, hashlib, requests

RPC = "https://rpc.mainnet.x1.xyz"

def rpc_call(method, params):
    resp = requests.post(RPC, json={"jsonrpc": "2.0", "id": 1, "method": method, "params": params})
    return resp.json()["result"]

def anchor_discriminator(name):
    """Compute Anchor instruction discriminator: SHA256('global:<name>')[..8]"""
    return hashlib.sha256(f"global:{name}".encode()).digest()[:8]

def build_and_send_mint_tx(keypair, memo_text):
    """Example: Build, sign, and send a mint transaction"""
    user = keypair.pubkey()

    MINT_PROGRAM = Pubkey.from_string("8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX")
    MEMO_MINT = Pubkey.from_string("memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick")
    TOKEN_2022 = Pubkey.from_string("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
    SPL_MEMO = Pubkey.from_string("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
    INSTRUCTIONS_SYSVAR = Pubkey.from_string("Sysvar1nstructions1111111111111111111111111")

    # Derive PDAs
    mint_authority, _ = Pubkey.find_program_address([b"mint_authority"], MINT_PROGRAM)
    # Derive user's MEMO ATA (Token-2022)
    ATA_PROGRAM = Pubkey.from_string("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
    user_ata, _ = Pubkey.find_program_address(
        [bytes(user), bytes(TOKEN_2022), bytes(MEMO_MINT)], ATA_PROGRAM
    )

    # 1. SPL Memo instruction (must be index 0)
    memo_ix = Instruction(
        program_id=SPL_MEMO,
        accounts=[AccountMeta(user, is_signer=True, is_writable=True)],
        data=memo_text.encode('utf-8')
    )

    # 2. Mint instruction
    mint_ix = Instruction(
        program_id=MINT_PROGRAM,
        accounts=[
            AccountMeta(user, is_signer=True, is_writable=True),
            AccountMeta(MEMO_MINT, is_signer=False, is_writable=True),
            AccountMeta(mint_authority, is_signer=False, is_writable=False),
            AccountMeta(user_ata, is_signer=False, is_writable=True),
            AccountMeta(TOKEN_2022, is_signer=False, is_writable=False),
            AccountMeta(INSTRUCTIONS_SYSVAR, is_signer=False, is_writable=False),
        ],
        data=bytes(anchor_discriminator("process_mint"))
    )

    # 3. Get blockhash
    blockhash_resp = rpc_call("getLatestBlockhash", [{"commitment": "confirmed"}])
    blockhash = Hash.from_string(blockhash_resp["value"]["blockhash"])

    # 4. Add compute budget
    cu_ix = set_compute_unit_limit(400_000)

    # 5. Build, sign, send
    msg = Message.new_with_payer([memo_ix, mint_ix, cu_ix], user)
    tx = Transaction.new([keypair], msg, blockhash)
    tx_b64 = base64.b64encode(bytes(tx)).decode('utf-8')

    result = rpc_call("sendTransaction", [
        tx_b64, {"encoding": "base64", "preflightCommitment": "confirmed", "maxRetries": 3}
    ])
    return result  # Transaction signature
```

---

## Error Handling

RPC errors follow this structure:
```json
{
  "error": {
    "code": -32002,
    "message": "Transaction simulation failed: ...",
    "data": {
      "err": {"InstructionError": [0, {"Custom": 6001}]},
      "logs": ["Program log: Error Message: Memo too short"]
    }
  }
}
```

Extract specific error messages from `data.logs` entries containing `"Error Message:"`.

---

## Quick Reference: Common Queries

| Task | Method | Key Param |
|---|---|---|
| Check XNT balance | `getBalance` | pubkey |
| Check MEMO balance | `getTokenAccountsByOwner` | owner + mint filter |
| MEMO total supply | `getTokenSupply` | MEMO mint address |
| User profile | `getAccountInfo` | Profile PDA |
| Token holders | `getProgramAccounts` | Token-2022 + memcmp mint |
| Top burners | `getProgramAccounts` | Burn program + dataSize:65 |
| Tx history | `getSignaturesForAddress` | address + limit |
| Tx details | `getTransaction` | signature |
| DEX pools | `getProgramAccounts` | xDEX program |
| Pool price | `getAccountInfo` on vaults | vault addresses |
| Health check | `getVersion` | (none) |

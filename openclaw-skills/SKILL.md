---
name: openclaw-x1-memo-protocol
description: Interact with the MEMO Protocol on X1 blockchain via @solana/web3.js. Covers the full MEMO ecosystem including token minting, burning, transfers, user profiles, chat groups, forum posts, projects, and blogs. Use when querying on-chain data (balances, token holders, burn stats, profiles, posts, blogs, projects) or building transactions against X1 mainnet/testnet.
---

# OpenClaw X1 MEMO Protocol Skill

Direct interaction with the MEMO Protocol on X1 blockchain using `@solana/web3.js`. The X1 RPC is public and free — no API keys needed.

## Setup & Dependencies

```bash
npm install @solana/web3.js @solana/spl-token
```

```javascript
import {
  Connection, PublicKey, Keypair, Transaction,
  TransactionInstruction, SystemProgram, ComputeBudgetProgram,
  sendAndConfirmTransaction
} from '@solana/web3.js';
import {
  getAssociatedTokenAddress, createAssociatedTokenAccountInstruction,
  createTransferCheckedInstruction, TOKEN_2022_PROGRAM_ID
} from '@solana/spl-token';
import { createHash } from 'crypto';

const connection = new Connection('https://rpc.mainnet.x1.xyz', 'confirmed');
```

---

## Program IDs & Token Addresses

```javascript
// MEMO Protocol Programs
const MINT_PROGRAM       = new PublicKey('8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX');
const BURN_PROGRAM       = new PublicKey('2sb3gz5Cmr2g1ia5si2rmCZqPACxgaZXEmiS5k6Htcvh');
const CHAT_PROGRAM       = new PublicKey('Hni4qE8GGW5uwBWzUEkpPBDRwXvKCWhM96teieAReRyd');
const PROFILE_PROGRAM    = new PublicKey('2BY8vPpQRFFwAqK3HqU5qL3qsGMH3VnX9Gv9bud3vzH8');
const PROJECT_PROGRAM    = new PublicKey('6Vavot6ybhWBG3rjNXnLfNRPVTz7Garf6E4EZk3byp3a');
const BLOG_PROGRAM       = new PublicKey('3EKdp88FgyPC41bxRDzFAtCDUMV2g9SVt5UiytE8wdzM');
const FORUM_PROGRAM      = new PublicKey('6gzhG5BveTkJfTi466toX4qmN3BtU9qp1Grnk61GvmXD');

// Token Addresses
const MEMO_MINT          = new PublicKey('memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick');   // Token-2022, 6 decimals
const SPL_MEMO_PROGRAM   = new PublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr');
const ATA_PROGRAM        = new PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL');
const INSTRUCTIONS_SYSVAR = new PublicKey('Sysvar1nstructions1111111111111111111111111');

// XNT (native token): 9 decimals
// MEMO token: 6 decimals (1 MEMO = 1_000_000 lamports)
```

### RPC Endpoints

```
Mainnet: https://rpc.mainnet.x1.xyz
Testnet: https://rpc.testnet.x1.xyz
```

### MEMO Supply Tiers (Mint Reward Schedule)

**Maximum Supply Cap: 10 trillion (10,000,000,000,000) MEMO tokens**. When total supply reaches this cap, no more tokens can be minted.

| Supply Range | Reward per Mint |
|---|---|
| 0 - 100M | 1.0 MEMO |
| 100M - 1B | 0.1 MEMO |
| 1B - 10B | 0.01 MEMO |
| 10B - 100B | 0.001 MEMO |
| 100B - 1T | 0.0001 MEMO |
| 1T - 10T | 0.000001 MEMO (1 lamport) |

---

## Utility Functions

### Borsh Encoding/Decoding Helpers

```javascript
// ── Encoding ──

function encodeBorshU8(value) {
  return Buffer.from([value]);
}

function encodeBorshU64(value) {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(BigInt(value));
  return buf;
}

function encodeBorshI64(value) {
  const buf = Buffer.alloc(8);
  buf.writeBigInt64LE(BigInt(value));
  return buf;
}

function encodeBorshString(str) {
  const bytes = Buffer.from(str, 'utf-8');
  const len = Buffer.alloc(4);
  len.writeUInt32LE(bytes.length);
  return Buffer.concat([len, bytes]);
}

function encodeBorshOptionString(value) {
  if (value === null || value === undefined) return Buffer.from([0]);
  return Buffer.concat([Buffer.from([1]), encodeBorshString(value)]);
}

function encodeBorshOptionI64(value) {
  if (value === null || value === undefined) return Buffer.from([0]);
  return Buffer.concat([Buffer.from([1]), encodeBorshI64(value)]);
}

function encodeBorshVecString(arr) {
  const len = Buffer.alloc(4);
  len.writeUInt32LE(arr.length);
  return Buffer.concat([len, ...arr.map(encodeBorshString)]);
}

function encodeBorshVecU8(data) {
  const len = Buffer.alloc(4);
  len.writeUInt32LE(data.length);
  return Buffer.concat([len, data]);
}

// ── Decoding ──

function decodeBorshU8(data, offset) {
  return [data[offset], offset + 1];
}

function decodeBorshU64(data, offset) {
  const value = data.readBigUInt64LE(offset);
  return [Number(value), offset + 8];
}

function decodeBorshI64(data, offset) {
  const value = data.readBigInt64LE(offset);
  return [Number(value), offset + 8];
}

function decodeBorshString(data, offset) {
  const len = data.readUInt32LE(offset);
  offset += 4;
  const str = data.slice(offset, offset + len).toString('utf-8');
  return [str, offset + len];
}

function decodeBorshOptionString(data, offset) {
  const flag = data[offset]; offset += 1;
  if (flag === 0) return [null, offset];
  return decodeBorshString(data, offset);
}

function decodeBorshVecString(data, offset) {
  const count = data.readUInt32LE(offset); offset += 4;
  const arr = [];
  for (let i = 0; i < count; i++) {
    const [str, newOffset] = decodeBorshString(data, offset);
    arr.push(str); offset = newOffset;
  }
  return [arr, offset];
}
```

### Anchor Instruction Discriminator

```javascript
function anchorDiscriminator(instructionName) {
  const hash = createHash('sha256')
    .update(`global:${instructionName}`)
    .digest();
  return hash.slice(0, 8);
}
```

### PDA Derivation Patterns

```javascript
// Mint Authority
const [mintAuthority] = PublicKey.findProgramAddressSync(
  [Buffer.from('mint_authority')], MINT_PROGRAM
);

// User Profile
const [profilePda] = PublicKey.findProgramAddressSync(
  [Buffer.from('profile'), userPubkey.toBuffer()], PROFILE_PROGRAM
);

// User Burn Stats
const [burnStatsPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('user_global_burn_stats'), userPubkey.toBuffer()], BURN_PROGRAM
);

// Chat Group
const groupIdBuf = Buffer.alloc(8); groupIdBuf.writeBigUInt64LE(BigInt(groupId));
const [chatGroupPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('chat_group'), groupIdBuf], CHAT_PROGRAM
);

// Chat Global Counter
const [chatGlobalCounter] = PublicKey.findProgramAddressSync(
  [Buffer.from('global_counter')], CHAT_PROGRAM
);

// Chat Burn Leaderboard
const [chatBurnLeaderboard] = PublicKey.findProgramAddressSync(
  [Buffer.from('burn_leaderboard')], CHAT_PROGRAM
);

// Forum Post
const postIdBuf = Buffer.alloc(8); postIdBuf.writeBigUInt64LE(BigInt(postId));
const [postPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('post'), postIdBuf], FORUM_PROGRAM
);

// Forum Global Counter
const [forumGlobalCounter] = PublicKey.findProgramAddressSync(
  [Buffer.from('global_counter')], FORUM_PROGRAM
);

// Blog (one per user)
const [blogPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('blog'), userPubkey.toBuffer()], BLOG_PROGRAM
);

// Project
const projectIdBuf = Buffer.alloc(8); projectIdBuf.writeBigUInt64LE(BigInt(projectId));
const [projectPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('project'), projectIdBuf], PROJECT_PROGRAM
);

// Project Global Counter
const [projectGlobalCounter] = PublicKey.findProgramAddressSync(
  [Buffer.from('global_counter')], PROJECT_PROGRAM
);

// Project Burn Leaderboard
const [projectBurnLeaderboard] = PublicKey.findProgramAddressSync(
  [Buffer.from('burn_leaderboard')], PROJECT_PROGRAM
);

// Token ATA (Token-2022)
const userAta = await getAssociatedTokenAddress(
  MEMO_MINT, userPubkey, false, TOKEN_2022_PROGRAM_ID
);
```

### PDA Quick Reference Table

| Account Type | Seeds | Program |
|---|---|---|
| Mint Authority | `["mint_authority"]` | Mint Program |
| User Profile | `["profile", user_pubkey]` | Profile Program |
| User Burn Stats | `["user_global_burn_stats", user_pubkey]` | Burn Program |
| Chat Group | `["chat_group", group_id (u64 LE)]` | Chat Program |
| Chat Global Counter | `["global_counter"]` | Chat Program |
| Chat Burn Leaderboard | `["burn_leaderboard"]` | Chat Program |
| Forum Post | `["post", post_id (u64 LE)]` | Forum Program |
| Forum Global Counter | `["global_counter"]` | Forum Program |
| Blog | `["blog", user_pubkey]` | Blog Program |
| Project | `["project", project_id (u64 LE)]` | Project Program |
| Project Global Counter | `["global_counter"]` | Project Program |
| Project Burn Leaderboard | `["burn_leaderboard"]` | Project Program |
| Token ATA | `[owner, token_program, mint]` | ATA Program |

---

## Read Operations

### 1. Get XNT Balance

```javascript
const balance = await connection.getBalance(new PublicKey(address));
const xnt = balance / 1_000_000_000;  // 9 decimals
```

### 2. Get MEMO Token Balance

```javascript
const accounts = await connection.getTokenAccountsByOwner(
  new PublicKey(ownerAddress),
  { mint: MEMO_MINT },
  { encoding: 'jsonParsed' }
);
if (accounts.value.length > 0) {
  const balance = accounts.value[0].account.data.parsed.info.tokenAmount.uiAmount;
}
```

### 3. Get MEMO Token Supply

```javascript
const supply = await connection.getTokenSupply(MEMO_MINT);
// supply.value.uiAmount → human-readable (6 decimals)
// supply.value.amount   → raw lamports string
```

### 4. Get Account Info

```javascript
const accountInfo = await connection.getAccountInfo(new PublicKey(address));
// accountInfo.data → Buffer containing account data
```

### 5. Get Transaction Details

```javascript
const tx = await connection.getTransaction(signature, {
  maxSupportedTransactionVersion: 0,
  commitment: 'confirmed'
});
// tx.meta.logMessages → program logs
// tx.transaction.message.instructions → instructions
```

### 6. Get Transaction History

```javascript
const signatures = await connection.getSignaturesForAddress(
  new PublicKey(address),
  { limit: 20 }
);
// signatures[i].signature → transaction signature
// signatures[i].memo → memo field (if SPL Memo was used)
```

### 7. Get Top MEMO Token Holders

```javascript
const accounts = await connection.getProgramAccounts(TOKEN_2022_PROGRAM_ID, {
  encoding: 'jsonParsed',
  filters: [
    { memcmp: { offset: 0, bytes: MEMO_MINT.toBase58() } }
  ]
});

const holders = accounts.map(a => ({
  owner: a.account.data.parsed.info.owner,
  balance: a.account.data.parsed.info.tokenAmount.uiAmount
})).sort((a, b) => b.balance - a.balance);
```

### 8. Get Top Burners

```javascript
const accounts = await connection.getProgramAccounts(BURN_PROGRAM, {
  encoding: 'base64',
  filters: [{ dataSize: 65 }]
});

const burners = accounts.map(({ account }) => {
  const data = Buffer.from(account.data[0], 'base64');
  // Skip 8-byte discriminator
  const user = new PublicKey(data.slice(8, 40)).toBase58();
  const totalBurned = Number(data.readBigUInt64LE(40)) / 1_000_000;
  const burnCount = Number(data.readBigUInt64LE(48));
  const lastBurnTime = Number(data.readBigInt64LE(56));
  return { user, totalBurned, burnCount, lastBurnTime };
}).sort((a, b) => b.totalBurned - a.totalBurned);
```

### 9. Get User Profile

```javascript
function parseProfile(data) {
  let offset = 8; // skip discriminator
  const user = new PublicKey(data.slice(offset, offset + 32)).toBase58(); offset += 32;
  let username; [username, offset] = decodeBorshString(data, offset);
  let image;    [image, offset]    = decodeBorshString(data, offset);
  let createdAt;  [createdAt, offset]  = decodeBorshI64(data, offset);
  let lastUpdated; [lastUpdated, offset] = decodeBorshI64(data, offset);
  let aboutMe;  [aboutMe, offset]  = decodeBorshOptionString(data, offset);
  const bump = data[offset];
  return { user, username, image, createdAt, lastUpdated, aboutMe, bump };
}

const [profilePda] = PublicKey.findProgramAddressSync(
  [Buffer.from('profile'), userPubkey.toBuffer()], PROFILE_PROGRAM
);
const accountInfo = await connection.getAccountInfo(profilePda);
if (accountInfo) {
  const profile = parseProfile(Buffer.from(accountInfo.data));
  // profile.image → pixel art string (see Pixel Art section)
}
```

### 10. Get Forum Post

```javascript
function parseForumPost(data) {
  let offset = 8; // skip discriminator
  let postId;     [postId, offset]     = decodeBorshU64(data, offset);
  const creator = new PublicKey(data.slice(offset, offset + 32)).toBase58(); offset += 32;
  let createdAt;  [createdAt, offset]  = decodeBorshI64(data, offset);
  let lastUpdated; [lastUpdated, offset] = decodeBorshI64(data, offset);
  let title;      [title, offset]      = decodeBorshString(data, offset);
  let content;    [content, offset]    = decodeBorshString(data, offset);
  let image;      [image, offset]      = decodeBorshString(data, offset);
  let replyCount; [replyCount, offset] = decodeBorshU64(data, offset);
  let burnedAmount; [burnedAmount, offset] = decodeBorshU64(data, offset);
  let lastReplyTime; [lastReplyTime, offset] = decodeBorshI64(data, offset);
  const bump = data[offset];
  return { postId, creator, createdAt, lastUpdated, title, content, image,
           replyCount, burnedAmount: burnedAmount / 1_000_000, lastReplyTime, bump };
}

const postIdBuf = Buffer.alloc(8); postIdBuf.writeBigUInt64LE(BigInt(postId));
const [postPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('post'), postIdBuf], FORUM_PROGRAM
);
const accountInfo = await connection.getAccountInfo(postPda);
if (accountInfo) {
  const post = parseForumPost(Buffer.from(accountInfo.data));
}
```

### 11. Get Blog

```javascript
function parseBlog(data) {
  let offset = 8; // skip discriminator
  const creator = new PublicKey(data.slice(offset, offset + 32)).toBase58(); offset += 32;
  let createdAt;  [createdAt, offset]  = decodeBorshI64(data, offset);
  let lastUpdated; [lastUpdated, offset] = decodeBorshI64(data, offset);
  let name;       [name, offset]       = decodeBorshString(data, offset);
  let description; [description, offset] = decodeBorshString(data, offset);
  let image;      [image, offset]      = decodeBorshString(data, offset);
  let memoCount;  [memoCount, offset]  = decodeBorshU64(data, offset);
  let burnedAmount; [burnedAmount, offset] = decodeBorshU64(data, offset);
  let lastMemoTime; [lastMemoTime, offset] = decodeBorshI64(data, offset);
  const bump = data[offset];
  return { creator, createdAt, lastUpdated, name, description, image,
           memoCount, burnedAmount: burnedAmount / 1_000_000, lastMemoTime, bump };
}

const [blogPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('blog'), userPubkey.toBuffer()], BLOG_PROGRAM
);
const accountInfo = await connection.getAccountInfo(blogPda);
if (accountInfo) {
  const blog = parseBlog(Buffer.from(accountInfo.data));
}
```

### 12. Get Project

```javascript
function parseProject(data) {
  let offset = 8; // skip discriminator
  let projectId;  [projectId, offset]  = decodeBorshU64(data, offset);
  const creator = new PublicKey(data.slice(offset, offset + 32)).toBase58(); offset += 32;
  let createdAt;  [createdAt, offset]  = decodeBorshI64(data, offset);
  let lastUpdated; [lastUpdated, offset] = decodeBorshI64(data, offset);
  let name;       [name, offset]       = decodeBorshString(data, offset);
  let description; [description, offset] = decodeBorshString(data, offset);
  let image;      [image, offset]      = decodeBorshString(data, offset);
  let website;    [website, offset]    = decodeBorshString(data, offset);
  let tags;       [tags, offset]       = decodeBorshVecString(data, offset);
  let memoCount;  [memoCount, offset]  = decodeBorshU64(data, offset);
  let burnedAmount; [burnedAmount, offset] = decodeBorshU64(data, offset);
  let lastMemoTime; [lastMemoTime, offset] = decodeBorshI64(data, offset);
  const bump = data[offset];
  return { projectId, creator, createdAt, lastUpdated, name, description, image,
           website, tags, memoCount, burnedAmount: burnedAmount / 1_000_000, lastMemoTime, bump };
}

const projectIdBuf = Buffer.alloc(8); projectIdBuf.writeBigUInt64LE(BigInt(projectId));
const [projectPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('project'), projectIdBuf], PROJECT_PROGRAM
);
const accountInfo = await connection.getAccountInfo(projectPda);
if (accountInfo) {
  const project = parseProject(Buffer.from(accountInfo.data));
}
```

### 13. Get Chat Group

```javascript
function parseChatGroup(data) {
  let offset = 8; // skip discriminator
  let groupId;    [groupId, offset]    = decodeBorshU64(data, offset);
  const creator = new PublicKey(data.slice(offset, offset + 32)).toBase58(); offset += 32;
  let createdAt;  [createdAt, offset]  = decodeBorshI64(data, offset);
  let name;       [name, offset]       = decodeBorshString(data, offset);
  let description; [description, offset] = decodeBorshString(data, offset);
  let image;      [image, offset]      = decodeBorshString(data, offset);
  let tags;       [tags, offset]       = decodeBorshVecString(data, offset);
  let memoCount;  [memoCount, offset]  = decodeBorshU64(data, offset);
  let burnedAmount; [burnedAmount, offset] = decodeBorshU64(data, offset);
  let minMemoInterval; [minMemoInterval, offset] = decodeBorshI64(data, offset);
  let lastMemoTime; [lastMemoTime, offset] = decodeBorshI64(data, offset);
  const bump = data[offset];
  return { groupId, creator, createdAt, name, description, image, tags,
           memoCount, burnedAmount: burnedAmount / 1_000_000, minMemoInterval, lastMemoTime, bump };
}

const groupIdBuf = Buffer.alloc(8); groupIdBuf.writeBigUInt64LE(BigInt(groupId));
const [chatGroupPda] = PublicKey.findProgramAddressSync(
  [Buffer.from('chat_group'), groupIdBuf], CHAT_PROGRAM
);
const accountInfo = await connection.getAccountInfo(chatGroupPda);
if (accountInfo) {
  const group = parseChatGroup(Buffer.from(accountInfo.data));
}
```

### 14. Get Global Counter (Forum/Project/Chat)

```javascript
function parseGlobalCounter(data) {
  // Offset 0-7: discriminator, Offset 8-15: count (u64 LE)
  return Number(data.readBigUInt64LE(8));
}

// Forum total posts
const [forumCounter] = PublicKey.findProgramAddressSync(
  [Buffer.from('global_counter')], FORUM_PROGRAM
);
const forumInfo = await connection.getAccountInfo(forumCounter);
const totalPosts = parseGlobalCounter(Buffer.from(forumInfo.data));

// Project total projects — same pattern with PROJECT_PROGRAM
// Chat total groups — same pattern with CHAT_PROGRAM
```

### 15. Get Chat Messages (from Transaction History)

```javascript
async function getChatMessages(groupId, limit = 20) {
  const groupIdBuf = Buffer.alloc(8); groupIdBuf.writeBigUInt64LE(BigInt(groupId));
  const [chatGroupPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('chat_group'), groupIdBuf], CHAT_PROGRAM
  );

  const sigs = await connection.getSignaturesForAddress(chatGroupPda, { limit });
  const messages = [];

  for (const sig of sigs) {
    if (!sig.memo) continue;

    // Strip "[length] " prefix from memo field
    const memoStr = sig.memo.includes(' ') ? sig.memo.split(' ').slice(1).join(' ') : sig.memo;
    const borshBytes = Buffer.from(memoStr, 'base64');

    try {
      // Try parsing as ChatMessageData (direct Borsh+Base64)
      const parsed = decodeChatMessage(borshBytes);
      if (parsed) messages.push({ ...parsed, signature: sig.signature, blockTime: sig.blockTime });
    } catch (e) {
      // Try parsing as BurnMemo (for burn operations)
      try {
        const parsed = decodeBurnMemo(borshBytes);
        if (parsed) messages.push({ ...parsed, signature: sig.signature, blockTime: sig.blockTime });
      } catch (e2) { /* skip unparseable */ }
    }
  }
  return messages;
}
```

### 16. RPC Health Check

```javascript
const version = await connection.getVersion();
// version['solana-core'] → node version
```

---

## Memo Serialization & Deserialization

All MEMO Protocol operations encode structured data into SPL Memo instructions. There are two encoding patterns:

### Pattern A: Chat Messages (no burn, direct Borsh+Base64)

```
ChatMessageData struct
    │ Borsh serialize
    ▼
Binary bytes
    │ Base64 encode
    ▼
UTF-8 string → SPL Memo instruction data
```

### Pattern B: Burn Operations (profile, group, forum, blog, project)

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
UTF-8 string → SPL Memo instruction data
```

### Memo Constraints

- Minimum memo length: 69 bytes (Base64 string length)
- Maximum memo length: 800 bytes (Base64 string length)
- SPL Memo instruction must be at **index 0** in the transaction

### Borsh Encoding Reference

| Type | Borsh Binary Layout | Example |
|---|---|---|
| `u8` | 1 byte | `0x01` → 1 |
| `u64` | 8 bytes, little-endian | `0x40420F0000000000` → 1,000,000 |
| `i64` | 8 bytes, little-endian, signed | same as u64 but signed |
| `String` | 4-byte LE length + UTF-8 bytes | `0x05000000` + `hello` |
| `Vec<T>` | 4-byte LE count + N items | `0x02000000` + item1 + item2 |
| `Option<T>` | `0x00` (None) or `0x01` + T (Some) | `0x00` → None |
| `bool` | 1 byte | `0x00` → false, `0x01` → true |

---

## Data Structures (Borsh Field Order)

### BurnMemo (outer wrapper for Pattern B)

```
Field          Type       Notes
version        u8         always 1
burn_amount    u64        in lamports (÷ 1_000_000 for MEMO tokens)
payload        Vec<u8>    inner Borsh-serialized struct
```

### ChatMessageData (Pattern A — direct Borsh+Base64)

```
Field          Type              Values
version        u8                1
category       String            "chat"
operation      String            "send_message"
group_id       u64               target group ID
sender         String            Base58 pubkey string
message        String            1-512 chars
receiver       Option<String>    Base58 pubkey or null
reply_to_sig   Option<String>    tx signature or null
```

### ProfileCreationData (BurnMemo payload)

```
Field          Type              Values
version        u8                1
category       String            "profile"
operation      String            "create_profile"
user_pubkey    String            Base58 pubkey string
username       String            1-32 chars
image          String            max 256 chars (pixel art)
about_me       Option<String>    max 128 chars
```

### ProfileUpdateData (BurnMemo payload)

```
Field          Type                    Values
version        u8                      1
category       String                  "profile"
operation      String                  "update_profile"
user_pubkey    String                  Base58 pubkey string
username       Option<String>          1-32 chars
image          Option<String>          max 256 chars (pixel art)
about_me       Option<Option<String>>  nested Option (for clearing)
```

### ChatGroupCreationData (BurnMemo payload)

```
Field              Type              Values
version            u8                1
category           String            "chat"
operation          String            "create_group"
group_id           u64               expected group ID
name               String            1-64 chars
description        String            max 128 chars
image              String            max 256 chars (pixel art)
tags               Vec<String>       max 4 tags, each max 32 chars
min_memo_interval  Option<i64>       seconds (contract defaults to 60)
```

### ChatGroupBurnData (BurnMemo payload)

```
Field          Type       Values
version        u8         1
category       String     "chat"
operation      String     "burn_for_group"
group_id       u64        target group ID
burner         String     Base58 pubkey string
message        String     max 512 chars
```

### PostCreationData (BurnMemo payload — Forum)

```
Field          Type       Values
version        u8         1
category       String     "forum"
operation      String     "create_post"
creator        String     Base58 pubkey string
post_id        u64        expected post ID
title          String     1-128 chars
content        String     1-512 chars
image          String     max 256 chars (pixel art)
```

### PostBurnData (BurnMemo payload — Forum)

```
Field          Type       Values
version        u8         1
category       String     "forum"
operation      String     "burn_for_post"
user           String     Base58 pubkey string
post_id        u64        target post ID
message        String     max 512 chars
```

### PostMintData (BurnMemo payload — Forum)

```
Field          Type       Values
version        u8         1
category       String     "forum"
operation      String     "mint_for_post"
user           String     Base58 pubkey string
post_id        u64        target post ID
message        String     max 512 chars
```

### BlogCreationData (BurnMemo payload)

```
Field          Type       Values
version        u8         1
category       String     "blog"
operation      String     "create_blog"
creator        String     Base58 pubkey string
name           String     1-64 chars
description    String     max 256 chars
image          String     max 256 chars (pixel art)
```

### BlogUpdateData (BurnMemo payload)

```
Field          Type              Values
version        u8                1
category       String            "blog"
operation      String            "update_blog"
creator        String            Base58 pubkey string
name           Option<String>    1-64 chars
description    Option<String>    max 256 chars
image          Option<String>    max 256 chars (pixel art)
```

### BlogBurnData (BurnMemo payload)

```
Field          Type       Values
version        u8         1
category       String     "blog"
operation      String     "burn_for_blog"
burner         String     Base58 pubkey string
message        String     max 696 chars
```

### BlogMintData (BurnMemo payload)

```
Field          Type       Values
version        u8         1
category       String     "blog"
operation      String     "mint_for_blog"
minter         String     Base58 pubkey string
message        String     max 696 chars
```

### ProjectCreationData (BurnMemo payload)

```
Field          Type           Values
version        u8             1
category       String         "project"
operation      String         "create_project"
project_id     u64            expected project ID
name           String         1-64 chars
description    String         max 256 chars
image          String         max 256 chars (pixel art)
website        String         max 128 chars
tags           Vec<String>    max 4 tags, each 1-32 chars
```

### ProjectUpdateData (BurnMemo payload)

```
Field          Type                Values
version        u8                  1
category       String              "project"
operation      String              "update_project"
project_id     u64                 target project ID
name           Option<String>      1-64 chars
description    Option<String>      max 256 chars
image          Option<String>      max 256 chars (pixel art)
website        Option<String>      max 128 chars
tags           Option<Vec<String>> max 4 tags, each 1-32 chars
```

### ProjectBurnData (BurnMemo payload)

```
Field          Type       Values
version        u8         1
category       String     "project"
operation      String     "burn_for_project"
project_id     u64        target project ID
burner         String     Base58 pubkey string
message        String     max 696 chars
```

---

## Memo Encoding & Decoding Examples

### Encode a Chat Message (Pattern A)

```javascript
function encodeChatMessage(groupId, senderPubkey, message, receiver = null, replyToSig = null) {
  const data = Buffer.concat([
    encodeBorshU8(1),                          // version
    encodeBorshString('chat'),                 // category
    encodeBorshString('send_message'),         // operation
    encodeBorshU64(groupId),                   // group_id
    encodeBorshString(senderPubkey),           // sender
    encodeBorshString(message),                // message
    encodeBorshOptionString(receiver),         // receiver
    encodeBorshOptionString(replyToSig),       // reply_to_sig
  ]);
  return Buffer.from(data).toString('base64');
}

// Usage:
const memoBase64 = encodeChatMessage(1, keypair.publicKey.toBase58(), 'Hello world!');
```

### Encode a BurnMemo (Pattern B)

```javascript
function encodeBurnMemo(burnAmount, payloadBytes) {
  const memoBytes = Buffer.concat([
    encodeBorshU8(1),                          // version
    encodeBorshU64(burnAmount),                // burn_amount in lamports
    encodeBorshVecU8(payloadBytes),            // payload
  ]);
  return memoBytes.toString('base64');
}

// Example: Encode ProfileCreationData payload
function encodeProfileCreation(userPubkey, username, image, aboutMe = null) {
  return Buffer.concat([
    encodeBorshU8(1),                          // version
    encodeBorshString('profile'),              // category
    encodeBorshString('create_profile'),       // operation
    encodeBorshString(userPubkey),             // user_pubkey
    encodeBorshString(username),               // username
    encodeBorshString(image),                  // image (pixel art string)
    encodeBorshOptionString(aboutMe),          // about_me
  ]);
}

// Build full memo for profile creation (burn 420 MEMO)
const payload = encodeProfileCreation(
  keypair.publicKey.toBase58(), 'alice', 'n:32x32:###...', 'Hello, MEMO!'
);
const burnAmountLamports = 420 * 1_000_000; // 420 MEMO tokens
const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);
```

### Encode Forum Post Creation

```javascript
function encodePostCreation(creator, postId, title, content, image) {
  return Buffer.concat([
    encodeBorshU8(1),
    encodeBorshString('forum'),
    encodeBorshString('create_post'),
    encodeBorshString(creator),
    encodeBorshU64(postId),
    encodeBorshString(title),
    encodeBorshString(content),
    encodeBorshString(image),
  ]);
}

const payload = encodePostCreation(
  keypair.publicKey.toBase58(), nextPostId, 'My Post Title', 'Post content here...', 'n:32x32:###...'
);
const memoBase64 = encodeBurnMemo(1_000_000, payload); // burn 1 MEMO
```

### Encode Blog Creation

```javascript
function encodeBlogCreation(creator, name, description, image) {
  return Buffer.concat([
    encodeBorshU8(1),
    encodeBorshString('blog'),
    encodeBorshString('create_blog'),
    encodeBorshString(creator),
    encodeBorshString(name),
    encodeBorshString(description),
    encodeBorshString(image),
  ]);
}
```

### Encode Project Creation

```javascript
function encodeProjectCreation(projectId, name, description, image, website, tags) {
  return Buffer.concat([
    encodeBorshU8(1),
    encodeBorshString('project'),
    encodeBorshString('create_project'),
    encodeBorshU64(projectId),
    encodeBorshString(name),
    encodeBorshString(description),
    encodeBorshString(image),
    encodeBorshString(website),
    encodeBorshVecString(tags),
  ]);
}
```

### Encode Chat Group Creation

```javascript
function encodeChatGroupCreation(groupId, name, description, image, tags, minMemoInterval = null) {
  // NOTE: No 'creator' field — the contract identifies creator from the transaction signer
  return Buffer.concat([
    encodeBorshU8(1),
    encodeBorshString('chat'),
    encodeBorshString('create_group'),
    encodeBorshU64(groupId),
    encodeBorshString(name),
    encodeBorshString(description),
    encodeBorshString(image),
    encodeBorshVecString(tags),
    encodeBorshOptionI64(minMemoInterval),
  ]);
}
```

### Decode a Chat Message

```javascript
function decodeChatMessage(data) {
  let offset = 0;
  let version;   [version, offset]  = decodeBorshU8(data, offset);
  let category;  [category, offset] = decodeBorshString(data, offset);
  let operation; [operation, offset] = decodeBorshString(data, offset);

  if (category !== 'chat' || operation !== 'send_message') return null;

  let groupId;   [groupId, offset]  = decodeBorshU64(data, offset);
  let sender;    [sender, offset]   = decodeBorshString(data, offset);
  let message;   [message, offset]  = decodeBorshString(data, offset);
  let receiver;  [receiver, offset] = decodeBorshOptionString(data, offset);
  let replyTo;   [replyTo, offset]  = decodeBorshOptionString(data, offset);

  return { type: 'chat_message', groupId, sender, message, receiver, replyTo };
}
```

### Decode a BurnMemo

```javascript
function decodeBurnMemo(data) {
  let offset = 0;
  let version;     [version, offset]     = decodeBorshU8(data, offset);
  let burnAmount;  [burnAmount, offset]  = decodeBorshU64(data, offset);
  const payloadLen = data.readUInt32LE(offset); offset += 4;
  const payload = data.slice(offset, offset + payloadLen);

  // Parse inner payload
  let pOffset = 0;
  let pVersion;   [pVersion, pOffset]   = decodeBorshU8(payload, pOffset);
  let pCategory;  [pCategory, pOffset]  = decodeBorshString(payload, pOffset);
  let pOperation; [pOperation, pOffset] = decodeBorshString(payload, pOffset);

  const result = {
    burnAmount,
    burnTokens: burnAmount / 1_000_000,
    category: pCategory,
    operation: pOperation,
  };

  if (pCategory === 'profile') {
    let userPubkey; [userPubkey, pOffset] = decodeBorshString(payload, pOffset);
    let username;   [username, pOffset]   = decodeBorshString(payload, pOffset);
    let image;      [image, pOffset]      = decodeBorshString(payload, pOffset);
    let aboutMe;    [aboutMe, pOffset]    = decodeBorshOptionString(payload, pOffset);
    Object.assign(result, { type: 'profile', userPubkey, username, image, aboutMe });
  }
  else if (pCategory === 'chat' && pOperation === 'burn_for_group') {
    let groupId; [groupId, pOffset] = decodeBorshU64(payload, pOffset);
    let burner;  [burner, pOffset]  = decodeBorshString(payload, pOffset);
    let message; [message, pOffset] = decodeBorshString(payload, pOffset);
    Object.assign(result, { type: 'chat_burn', groupId, burner, message });
  }
  else if (pCategory === 'chat' && pOperation === 'create_group') {
    let groupId; [groupId, pOffset] = decodeBorshU64(payload, pOffset);
    let name;    [name, pOffset]    = decodeBorshString(payload, pOffset);
    Object.assign(result, { type: 'chat_group_creation', groupId, name });
  }
  else if (pCategory === 'forum' && pOperation === 'create_post') {
    let creator; [creator, pOffset] = decodeBorshString(payload, pOffset);
    let postId;  [postId, pOffset]  = decodeBorshU64(payload, pOffset);
    let title;   [title, pOffset]   = decodeBorshString(payload, pOffset);
    let content; [content, pOffset] = decodeBorshString(payload, pOffset);
    let image;   [image, pOffset]   = decodeBorshString(payload, pOffset);
    Object.assign(result, { type: 'forum_post', creator, postId, title, content, image });
  }
  else if (pCategory === 'blog') {
    let user; [user, pOffset] = decodeBorshString(payload, pOffset);
    let message; [message, pOffset] = decodeBorshString(payload, pOffset);
    Object.assign(result, { type: 'blog_' + pOperation, user, message });
  }
  else if (pCategory === 'project' && pOperation === 'burn_for_project') {
    let projectId; [projectId, pOffset] = decodeBorshU64(payload, pOffset);
    let burner;    [burner, pOffset]    = decodeBorshString(payload, pOffset);
    let message;   [message, pOffset]   = decodeBorshString(payload, pOffset);
    Object.assign(result, { type: 'project_burn', projectId, burner, message });
  }
  else {
    Object.assign(result, { type: 'unknown' });
  }

  return result;
}
```

### Decode Memo from Transaction History

```javascript
function decodeMemoFromSignature(memoField) {
  if (!memoField) return null;

  // Strip "[length] " prefix
  const memoStr = memoField.includes(' ') ? memoField.split(' ').slice(1).join(' ') : memoField;
  const borshBytes = Buffer.from(memoStr, 'base64');

  // Try Pattern A (ChatMessageData)
  try {
    const msg = decodeChatMessage(borshBytes);
    if (msg) return msg;
  } catch (e) {}

  // Try Pattern B (BurnMemo)
  try {
    const burn = decodeBurnMemo(borshBytes);
    if (burn) return burn;
  } catch (e) {}

  return null;
}
```

---

## Pixel Art Encoding (Profile & Entity Images)

The `image` field in profiles, chat groups, forum posts, blogs, and projects stores a **1-bit pixel art** encoded as a compact ASCII string. Each pixel is either black (1) or white (0).

### Storage Format

```
Normal:     "n:WxH:DATA"       e.g. "n:32x32:#####..."
Compressed: "c:WxH:BASE64"     e.g. "c:32x32:eJztwTEBAAAA..."
```

- `n` = normal (uncompressed safe-string)
- `c` = compressed (Deflate → Base64)
- `WxH` = width × height (e.g. `32x32`, `64x64`, `96x96`)
- Compression is only used when it produces a shorter result

Legacy format (backward compatible): `"n:DATA"` or `"c:BASE64"` (without WxH, auto-detects size from string length).

### Supported Sizes

| Size | Pixels | Safe String Chars |
|---|---|---|
| 8×8 | 64 | 11 |
| 16×16 | 256 | 43 |
| 32×32 | 1,024 | 171 |
| 64×64 | 4,096 | 683 |
| 96×96 | 9,216 | 1,536 |
| 128×128 | 16,384 | 2,731 |

### Safe String Character Mapping

Pixels are stored as a flat array in **row-major order** (left→right, top→bottom). 6 bits are packed per character.

**Encoding** (6-bit value → ASCII character):

```javascript
function encodePixelChar(value) {
  // value: 0-63
  let ascii = 35 + value;
  if (ascii >= 58) ascii += 1;  // skip ':' (ASCII 58)
  if (ascii >= 92) ascii += 1;  // skip '\' (ASCII 92)
  return String.fromCharCode(ascii);
}
```

**Decoding** (ASCII character → 6-bit value):

```javascript
function decodePixelChar(c) {
  const ascii = c.charCodeAt(0);
  if (ascii < 35 || ascii > 126) return null;
  if (c === ':' || c === '\\' || c === '"') return null;
  let value = ascii - 35;
  if (ascii > 92) value -= 1;  // adjust for skipped '\'
  if (ascii > 58) value -= 1;  // adjust for skipped ':'
  return value < 64 ? value : null;
}
```

### Complete Pixel Art Decoder (JavaScript)

```javascript
import { inflateRawSync } from 'zlib';

function decodePixelArt(imageString) {
  const parts = imageString.split(':');
  let fmt, width, height, data;

  if (parts.length === 3) {
    // New format: "n:WxH:data" or "c:WxH:data"
    fmt = parts[0];
    const [w, h] = parts[1].split('x').map(Number);
    width = w; height = h;
    data = parts[2];
  } else if (parts.length === 2) {
    // Legacy format: "n:data" or "c:data" (assume 32x32, auto-detect)
    fmt = parts[0];
    data = parts[1];
    width = 32; height = 32; // will auto-detect from string length
  } else {
    return null;
  }

  // Decompress if needed
  let safeString;
  if (fmt === 'c') {
    const compressed = Buffer.from(data, 'base64');
    safeString = inflateRawSync(compressed).toString('ascii');
  } else if (fmt === 'n') {
    safeString = data;
  } else {
    return null;
  }

  // Auto-detect size from string length if legacy format
  if (parts.length === 2) {
    const len = safeString.length;
    const SIZES = [8, 16, 32, 64, 96, 128, 256, 512, 1024];
    for (const s of SIZES) {
      if (Math.ceil(s * s / 6) === len) { width = s; height = s; break; }
    }
  }

  // Decode safe string to pixel bits
  const pixels = [];
  for (const c of safeString) {
    const value = decodePixelChar(c);
    if (value === null) return null;
    for (let i = 5; i >= 0; i--) {
      pixels.push(Boolean(value & (1 << i)));
    }
  }

  // Trim to exact pixel count and reshape to 2D grid
  const grid = [];
  for (let y = 0; y < height; y++) {
    grid.push(pixels.slice(y * width, (y + 1) * width));
  }
  return { width, height, grid }; // grid[row][col] = true (black) / false (white)
}
```

### Complete Pixel Art Encoder (JavaScript)

```javascript
import { deflateRawSync } from 'zlib';

function encodePixelArt(grid, width, height) {
  // Flatten grid to bit stream
  const bits = grid.flat();

  // Encode 6 bits at a time
  const chars = [];
  for (let i = 0; i < bits.length; i += 6) {
    const chunk = bits.slice(i, i + 6);
    let value = 0;
    for (const bit of chunk) {
      value = (value << 1) | (bit ? 1 : 0);
    }
    // Pad if chunk < 6 bits
    if (chunk.length < 6) {
      value <<= (6 - chunk.length);
    }
    chars.push(encodePixelChar(value));
  }

  const safeString = chars.join('');

  // Try compression
  const compressed = deflateRawSync(Buffer.from(safeString, 'ascii'));
  const compressedB64 = compressed.toString('base64');

  const normalResult = `n:${width}x${height}:${safeString}`;
  const compressedResult = `c:${width}x${height}:${compressedB64}`;

  return compressedResult.length < normalResult.length ? compressedResult : normalResult;
}

// Example: Create a 32x32 checkerboard
const grid = [];
for (let y = 0; y < 32; y++) {
  const row = [];
  for (let x = 0; x < 32; x++) {
    row.push((x + y) % 2 === 0);
  }
  grid.push(row);
}
const imageString = encodePixelArt(grid, 32, 32);
// Use this string in the 'image' field of ProfileCreationData, PostCreationData, etc.
```

### Rendering Pixel Art

- Each `true` = black pixel (filled), each `false` = white pixel (empty)
- Render as a square grid at the decoded `width × height`
- Display options: HTML Canvas, SVG, terminal block characters (`█` / ` `)

---

## Write Operations (Transaction Building)

All write operations follow this pattern:

1. **Build instructions** (SPL Memo instruction MUST be at index 0 when required)
2. **Get latest blockhash**
3. **Build, sign, and send transaction**
4. **Optionally simulate first** to estimate compute units

### SPL Memo Instruction Helper

```javascript
function createMemoInstruction(memoData, signer) {
  return new TransactionInstruction({
    keys: [{ pubkey: signer, isSigner: true, isWritable: true }],
    programId: SPL_MEMO_PROGRAM,
    data: Buffer.from(memoData, 'utf-8'),
  });
}
```

### Transaction Sending Helper

```javascript
async function buildAndSendTransaction(connection, keypair, instructions, computeUnits = 400_000) {
  const tx = new Transaction();

  // Add compute budget
  tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: computeUnits }));

  // Add all instructions
  for (const ix of instructions) {
    tx.add(ix);
  }

  const { blockhash } = await connection.getLatestBlockhash('confirmed');
  tx.recentBlockhash = blockhash;
  tx.feePayer = keypair.publicKey;

  const signature = await sendAndConfirmTransaction(connection, tx, [keypair], {
    commitment: 'confirmed',
    maxRetries: 3,
  });
  return signature;
}
```

### ATA Helper (Create if Needed)

```javascript
async function ensureATA(connection, payer, mint, owner, tokenProgram = TOKEN_2022_PROGRAM_ID) {
  const ata = await getAssociatedTokenAddress(mint, owner, false, tokenProgram);
  const account = await connection.getAccountInfo(ata);
  const instructions = [];
  if (!account) {
    instructions.push(
      createAssociatedTokenAccountInstruction(payer, ata, owner, mint, tokenProgram)
    );
  }
  return { ata, instructions };
}
```

---

### W1. Mint MEMO Token

Writes a memo on-chain and earns MEMO token rewards.

```javascript
async function mintMemo(connection, keypair, memoText) {
  const user = keypair.publicKey;

  // Derive PDAs
  const [mintAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from('mint_authority')], MINT_PROGRAM
  );
  const { ata: userAta, instructions: ataIxs } = await ensureATA(
    connection, user, MEMO_MINT, user
  );

  const instructions = [];

  // 1. SPL Memo instruction (MUST be at index 0)
  instructions.push(createMemoInstruction(memoText, user));

  // 2. Create ATA if needed
  instructions.push(...ataIxs);

  // 3. Mint instruction
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: mintAuthority,     isSigner: false, isWritable: false },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: MINT_PROGRAM,
    data: anchorDiscriminator('process_mint'),
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}

// Usage:
const sig = await mintMemo(connection, keypair, 'Hello, MEMO Protocol! This is my first memo on X1.');
```

**Memo content**: Plain text, 69-800 bytes. The user writes anything they want.

### W2. Transfer Native XNT

```javascript
async function transferXNT(connection, keypair, toAddress, amountXNT) {
  const lamports = Math.round(amountXNT * 1_000_000_000);
  const ix = SystemProgram.transfer({
    fromPubkey: keypair.publicKey,
    toPubkey: new PublicKey(toAddress),
    lamports,
  });
  return await buildAndSendTransaction(connection, keypair, [ix], 200_000);
}
```

### W3. Transfer MEMO Token (SPL Token-2022)

```javascript
async function transferMemo(connection, keypair, toAddress, amountMemo) {
  const user = keypair.publicKey;
  const destination = new PublicKey(toAddress);
  const amount = BigInt(Math.round(amountMemo * 1_000_000));

  const sourceAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);
  const { ata: destAta, instructions: ataIxs } = await ensureATA(
    connection, user, MEMO_MINT, destination
  );

  const instructions = [...ataIxs];
  instructions.push(createTransferCheckedInstruction(
    sourceAta, MEMO_MINT, destAta, user, amount, 6, [], TOKEN_2022_PROGRAM_ID
  ));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W4. Create User Profile

Burns MEMO tokens (minimum 420) to create an on-chain profile.

```javascript
async function createProfile(connection, keypair, username, image, aboutMe = null, burnAmount = 420) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  // Derive PDAs
  const [profilePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('profile'), user.toBuffer()], PROFILE_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  // Build memo payload
  const payload = encodeProfileCreation(user.toBase58(), username, image, aboutMe);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];

  // 1. SPL Memo (index 0)
  instructions.push(createMemoInstruction(memoBase64, user));

  // 2. Create profile instruction
  const ixData = Buffer.concat([
    anchorDiscriminator('create_profile'),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: profilePda,        isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,     isSigner: false, isWritable: false },
    ],
    programId: PROFILE_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W5. Update User Profile

```javascript
async function updateProfile(connection, keypair, username = null, image = null, aboutMe = undefined, burnAmount = 420) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const [profilePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('profile'), user.toBuffer()], PROFILE_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  // Build ProfileUpdateData payload
  const payload = Buffer.concat([
    encodeBorshU8(1),
    encodeBorshString('profile'),
    encodeBorshString('update_profile'),
    encodeBorshString(user.toBase58()),
    encodeBorshOptionString(username),
    encodeBorshOptionString(image),
    // about_me is Option<Option<String>>: 0x00=skip, 0x01+0x00=set to None, 0x01+0x01+String=set value
    aboutMe === undefined ? Buffer.from([0]) :
      aboutMe === null ? Buffer.from([1, 0]) :
      Buffer.concat([Buffer.from([1, 1]), encodeBorshString(aboutMe)]),
  ]);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('update_profile'),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: profilePda,        isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
    ],
    programId: PROFILE_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W6. Delete User Profile

```javascript
async function deleteProfile(connection, keypair) {
  const user = keypair.publicKey;
  const [profilePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('profile'), user.toBuffer()], PROFILE_PROGRAM
  );

  const ix = new TransactionInstruction({
    keys: [
      { pubkey: user,       isSigner: true,  isWritable: true  },
      { pubkey: profilePda, isSigner: false, isWritable: true  },
    ],
    programId: PROFILE_PROGRAM,
    data: anchorDiscriminator('delete_profile'),
  });

  return await buildAndSendTransaction(connection, keypair, [ix], 200_000);
}
```

### W7. Send Chat Message

Sends a message to a chat group and earns MEMO mint rewards.

```javascript
async function sendChatMessage(connection, keypair, groupId, message, receiver = null, replyToSig = null) {
  const user = keypair.publicKey;

  const groupIdBuf = Buffer.alloc(8); groupIdBuf.writeBigUInt64LE(BigInt(groupId));
  const [chatGroupPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('chat_group'), groupIdBuf], CHAT_PROGRAM
  );
  const [mintAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from('mint_authority')], MINT_PROGRAM
  );
  const { ata: userAta, instructions: ataIxs } = await ensureATA(
    connection, user, MEMO_MINT, user
  );

  // Encode memo (Pattern A: direct Borsh+Base64, no BurnMemo wrapper)
  const memoBase64 = encodeChatMessage(groupId, user.toBase58(), message, receiver, replyToSig);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));
  instructions.push(...ataIxs);

  const ixData = Buffer.concat([
    anchorDiscriminator('send_memo_to_group'),
    encodeBorshU64(groupId),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: chatGroupPda,      isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: mintAuthority,     isSigner: false, isWritable: false },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: MINT_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: CHAT_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W8. Create Chat Group

Burns MEMO tokens (minimum 42,069) to create a new chat group.

```javascript
async function createChatGroup(connection, keypair, name, description, image, tags = [], minMemoInterval = null, burnAmount = 42069) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  // Get expected group ID from global counter
  const [globalCounter] = PublicKey.findProgramAddressSync(
    [Buffer.from('global_counter')], CHAT_PROGRAM
  );
  const counterInfo = await connection.getAccountInfo(globalCounter);
  const expectedGroupId = Number(Buffer.from(counterInfo.data).readBigUInt64LE(8));

  const groupIdBuf = Buffer.alloc(8); groupIdBuf.writeBigUInt64LE(BigInt(expectedGroupId));
  const [chatGroupPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('chat_group'), groupIdBuf], CHAT_PROGRAM
  );
  const [burnLeaderboard] = PublicKey.findProgramAddressSync(
    [Buffer.from('burn_leaderboard')], CHAT_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = encodeChatGroupCreation(
    expectedGroupId, name, description, image, tags, minMemoInterval
  );
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('create_chat_group'),
    encodeBorshU64(expectedGroupId),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: globalCounter,     isSigner: false, isWritable: true  },
      { pubkey: chatGroupPda,      isSigner: false, isWritable: true  },
      { pubkey: burnLeaderboard,   isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,     isSigner: false, isWritable: false },
    ],
    programId: CHAT_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W9. Burn Tokens for Chat Group

```javascript
async function burnForGroup(connection, keypair, groupId, message, burnAmount = 1) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const groupIdBuf = Buffer.alloc(8); groupIdBuf.writeBigUInt64LE(BigInt(groupId));
  const [chatGroupPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('chat_group'), groupIdBuf], CHAT_PROGRAM
  );
  const [burnLeaderboard] = PublicKey.findProgramAddressSync(
    [Buffer.from('burn_leaderboard')], CHAT_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = Buffer.concat([
    encodeBorshU8(1), encodeBorshString('chat'), encodeBorshString('burn_for_group'),
    encodeBorshU64(groupId), encodeBorshString(user.toBase58()), encodeBorshString(message),
  ]);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('burn_tokens_for_group'),
    encodeBorshU64(groupId),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: chatGroupPda,      isSigner: false, isWritable: true  },
      { pubkey: burnLeaderboard,   isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: CHAT_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W10. Initialize Burn Stats (PREREQUISITE)

**IMPORTANT**: This must be executed ONCE per user BEFORE any burn operation (W4, W5, W8, W9, W12-W19). If the user's burn stats account does not exist, all burn transactions will fail. Query the stats PDA with `getAccountInfo` first — if the result is `null`, call this instruction before proceeding with any burn.

```javascript
async function initializeBurnStats(connection, keypair) {
  const user = keypair.publicKey;
  const [statsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );

  // Check if already initialized
  const existing = await connection.getAccountInfo(statsPda);
  if (existing) return null; // Already initialized

  const ix = new TransactionInstruction({
    keys: [
      { pubkey: user,     isSigner: true,  isWritable: true  },
      { pubkey: statsPda, isSigner: false, isWritable: true  },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: BURN_PROGRAM,
    data: anchorDiscriminator('initialize_user_global_burn_stats'),
  });

  return await buildAndSendTransaction(connection, keypair, [ix], 200_000);
}
```

### W11. Create Forum Post

Burns MEMO tokens (minimum 1) to create a forum post.

```javascript
async function createForumPost(connection, keypair, title, content, image, burnAmount = 1) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  // Get expected post ID
  const [globalCounter] = PublicKey.findProgramAddressSync(
    [Buffer.from('global_counter')], FORUM_PROGRAM
  );
  const counterInfo = await connection.getAccountInfo(globalCounter);
  const expectedPostId = Number(Buffer.from(counterInfo.data).readBigUInt64LE(8));

  const postIdBuf = Buffer.alloc(8); postIdBuf.writeBigUInt64LE(BigInt(expectedPostId));
  const [postPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('post'), postIdBuf], FORUM_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = encodePostCreation(user.toBase58(), expectedPostId, title, content, image);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('create_post'),
    encodeBorshU64(expectedPostId),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: globalCounter,     isSigner: false, isWritable: true  },
      { pubkey: postPda,           isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,     isSigner: false, isWritable: false },
    ],
    programId: FORUM_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W12. Burn for Forum Post

**ANY user** can burn tokens for any post (not restricted to creator). This is a key difference from Blog/Project.

```javascript
async function burnForPost(connection, keypair, postId, message, burnAmount = 1) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const postIdBuf = Buffer.alloc(8); postIdBuf.writeBigUInt64LE(BigInt(postId));
  const [postPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('post'), postIdBuf], FORUM_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = Buffer.concat([
    encodeBorshU8(1), encodeBorshString('forum'), encodeBorshString('burn_for_post'),
    encodeBorshString(user.toBase58()), encodeBorshU64(postId), encodeBorshString(message),
  ]);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('burn_for_post'),
    encodeBorshU64(postId),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: postPda,           isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: FORUM_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W13. Mint for Forum Post

**ANY user** can mint tokens for any post (not restricted to creator). This is a key difference from Blog/Project.

```javascript
async function mintForPost(connection, keypair, postId, message) {
  const user = keypair.publicKey;

  const postIdBuf = Buffer.alloc(8); postIdBuf.writeBigUInt64LE(BigInt(postId));
  const [postPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('post'), postIdBuf], FORUM_PROGRAM
  );
  const [mintAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from('mint_authority')], MINT_PROGRAM
  );
  const { ata: userAta, instructions: ataIxs } = await ensureATA(
    connection, user, MEMO_MINT, user
  );

  const payload = Buffer.concat([
    encodeBorshU8(1), encodeBorshString('forum'), encodeBorshString('mint_for_post'),
    encodeBorshString(user.toBase58()), encodeBorshU64(postId), encodeBorshString(message),
  ]);
  // mint operations: BurnMemo with burn_amount = 0
  const memoBase64 = encodeBurnMemo(0, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));
  instructions.push(...ataIxs);

  const ixData = Buffer.concat([
    anchorDiscriminator('mint_for_post'),
    encodeBorshU64(postId),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: postPda,           isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: mintAuthority,     isSigner: false, isWritable: false },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: MINT_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: FORUM_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W14. Create Blog

Burns MEMO tokens (minimum 1) to create a blog (one per user).

```javascript
async function createBlog(connection, keypair, name, description, image, burnAmount = 1) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const [blogPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('blog'), user.toBuffer()], BLOG_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = encodeBlogCreation(user.toBase58(), name, description, image);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('create_blog'),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: blogPda,           isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,     isSigner: false, isWritable: false },
    ],
    programId: BLOG_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W15. Update Blog

**Creator-only**: Only the blog creator can update their own blog.

```javascript
async function updateBlog(connection, keypair, name = null, description = null, image = null, burnAmount = 1) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const [blogPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('blog'), user.toBuffer()], BLOG_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = Buffer.concat([
    encodeBorshU8(1), encodeBorshString('blog'), encodeBorshString('update_blog'),
    encodeBorshString(user.toBase58()),
    encodeBorshOptionString(name), encodeBorshOptionString(description), encodeBorshOptionString(image),
  ]);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('update_blog'),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: blogPda,           isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: BLOG_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W16. Burn for Blog

**Creator-only**: Only the blog creator can burn for their own blog. The Blog PDA is derived from the burner's pubkey.

```javascript
async function burnForBlog(connection, keypair, blogOwnerPubkey, message, burnAmount = 1) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const [blogPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('blog'), new PublicKey(blogOwnerPubkey).toBuffer()], BLOG_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = Buffer.concat([
    encodeBorshU8(1), encodeBorshString('blog'), encodeBorshString('burn_for_blog'),
    encodeBorshString(user.toBase58()), encodeBorshString(message),
  ]);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('burn_for_blog'),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: blogPda,           isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: BLOG_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W17. Mint for Blog

**Creator-only**: Only the blog creator can mint for their own blog.

```javascript
async function mintForBlog(connection, keypair, blogOwnerPubkey, message) {
  const user = keypair.publicKey;

  const [blogPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('blog'), new PublicKey(blogOwnerPubkey).toBuffer()], BLOG_PROGRAM
  );
  const [mintAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from('mint_authority')], MINT_PROGRAM
  );
  const { ata: userAta, instructions: ataIxs } = await ensureATA(
    connection, user, MEMO_MINT, user
  );

  const payload = Buffer.concat([
    encodeBorshU8(1), encodeBorshString('blog'), encodeBorshString('mint_for_blog'),
    encodeBorshString(user.toBase58()), encodeBorshString(message),
  ]);
  const memoBase64 = encodeBurnMemo(0, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));
  instructions.push(...ataIxs);

  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: blogPda,           isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: mintAuthority,     isSigner: false, isWritable: false },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: MINT_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: BLOG_PROGRAM,
    data: anchorDiscriminator('mint_for_blog'),
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W18. Create Project

Burns MEMO tokens (minimum 42,069) to create a project.

```javascript
async function createProject(connection, keypair, name, description, image, website, tags = [], burnAmount = 42069) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const [globalCounter] = PublicKey.findProgramAddressSync(
    [Buffer.from('global_counter')], PROJECT_PROGRAM
  );
  const counterInfo = await connection.getAccountInfo(globalCounter);
  const expectedProjectId = Number(Buffer.from(counterInfo.data).readBigUInt64LE(8));

  const projectIdBuf = Buffer.alloc(8); projectIdBuf.writeBigUInt64LE(BigInt(expectedProjectId));
  const [projectPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('project'), projectIdBuf], PROJECT_PROGRAM
  );
  const [burnLeaderboard] = PublicKey.findProgramAddressSync(
    [Buffer.from('burn_leaderboard')], PROJECT_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = encodeProjectCreation(expectedProjectId, name, description, image, website, tags);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('create_project'),
    encodeBorshU64(expectedProjectId),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: globalCounter,     isSigner: false, isWritable: true  },
      { pubkey: projectPda,        isSigner: false, isWritable: true  },
      { pubkey: burnLeaderboard,   isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,     isSigner: false, isWritable: false },
    ],
    programId: PROJECT_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W19. Update Project

**Creator-only**: Only the project creator can update their own project.

```javascript
async function updateProject(connection, keypair, projectId, name = null, description = null, image = null, website = null, tags = null, burnAmount = 42069) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const projectIdBuf = Buffer.alloc(8); projectIdBuf.writeBigUInt64LE(BigInt(projectId));
  const [projectPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('project'), projectIdBuf], PROJECT_PROGRAM
  );
  const [burnLeaderboard] = PublicKey.findProgramAddressSync(
    [Buffer.from('burn_leaderboard')], PROJECT_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  // Encode Option<Vec<String>> for tags
  function encodeBorshOptionVecString(arr) {
    if (arr === null || arr === undefined) return Buffer.from([0]);
    return Buffer.concat([Buffer.from([1]), encodeBorshVecString(arr)]);
  }

  const payload = Buffer.concat([
    encodeBorshU8(1), encodeBorshString('project'), encodeBorshString('update_project'),
    encodeBorshU64(projectId),
    encodeBorshOptionString(name), encodeBorshOptionString(description),
    encodeBorshOptionString(image), encodeBorshOptionString(website),
    encodeBorshOptionVecString(tags),
  ]);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('update_project'),
    encodeBorshU64(projectId),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: projectPda,        isSigner: false, isWritable: true  },
      { pubkey: burnLeaderboard,   isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: PROJECT_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

### W20. Burn for Project

**Creator-only**: Only the project creator can burn for their own project.

```javascript
async function burnForProject(connection, keypair, projectId, message, burnAmount = 420) {
  const user = keypair.publicKey;
  const burnAmountLamports = burnAmount * 1_000_000;

  const projectIdBuf = Buffer.alloc(8); projectIdBuf.writeBigUInt64LE(BigInt(projectId));
  const [projectPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('project'), projectIdBuf], PROJECT_PROGRAM
  );
  const [burnLeaderboard] = PublicKey.findProgramAddressSync(
    [Buffer.from('burn_leaderboard')], PROJECT_PROGRAM
  );
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  const payload = Buffer.concat([
    encodeBorshU8(1), encodeBorshString('project'), encodeBorshString('burn_for_project'),
    encodeBorshU64(projectId), encodeBorshString(user.toBase58()), encodeBorshString(message),
  ]);
  const memoBase64 = encodeBurnMemo(burnAmountLamports, payload);

  const instructions = [];
  instructions.push(createMemoInstruction(memoBase64, user));

  const ixData = Buffer.concat([
    anchorDiscriminator('burn_for_project'),
    encodeBorshU64(projectId),
    encodeBorshU64(burnAmountLamports),
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },
      { pubkey: projectPda,        isSigner: false, isWritable: true  },
      { pubkey: burnLeaderboard,   isSigner: false, isWritable: true  },
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },
      { pubkey: userAta,           isSigner: false, isWritable: true  },
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: BURN_PROGRAM,      isSigner: false, isWritable: false },
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },
    ],
    programId: PROJECT_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}
```

---

## Minimum Burn Amounts

| Operation | Minimum MEMO | Lamports |
|---|---|---|
| Create Profile | 420 | 420,000,000 |
| Update Profile | 420 | 420,000,000 |
| Create Chat Group | 42,069 | 42,069,000,000 |
| Burn for Chat Group | 1 | 1,000,000 |
| Create Forum Post | 1 | 1,000,000 |
| Burn for Forum Post | 1 | 1,000,000 |
| Create Blog | 1 | 1,000,000 |
| Update Blog | 1 | 1,000,000 |
| Burn for Blog | 1 | 1,000,000 |
| Create Project | 42,069 | 42,069,000,000 |
| Update Project | 42,069 | 42,069,000,000 |
| Burn for Project | 420 | 420,000,000 |

---

## Access Control Summary

| Operation | Who Can Execute |
|---|---|
| Create Profile | Any user (for themselves) |
| Update Profile | Profile owner only |
| Delete Profile | Profile owner only |
| Create Chat Group | Any user |
| Send Chat Message | Any user (to any group) |
| Burn for Chat Group | Any user (for any group) |
| Create Forum Post | Any user |
| **Burn for Forum Post** | **Any user** (for any post) |
| **Mint for Forum Post** | **Any user** (for any post) |
| Create Blog | Any user (one blog per user) |
| Update Blog | **Blog creator only** |
| Burn for Blog | **Blog creator only** |
| Mint for Blog | **Blog creator only** |
| Create Project | Any user |
| Update Project | **Project creator only** |
| Burn for Project | **Project creator only** |

> **Key difference**: Forum posts allow any user to burn/mint (reply), while Blog and Project restrict burn/mint/update to the creator only.

---

## Error Handling

```javascript
try {
  const sig = await buildAndSendTransaction(connection, keypair, instructions);
} catch (error) {
  // Extract program error from logs
  if (error.logs) {
    const errorLog = error.logs.find(log => log.includes('Error Message:'));
    if (errorLog) console.error('Program error:', errorLog);
  }

  // Common error codes:
  // Custom(6001) → "Memo too short" (< 69 bytes)
  // Custom(6002) → "Memo too long" (> 800 bytes)
  // Custom(6003) → "Insufficient burn amount"
  // 0x1 (InsufficientFunds) → not enough SOL/XNT for fees
  // 0x1 (InsufficientFunds on token) → not enough MEMO tokens
}
```

---

## Instruction Discriminator Quick Reference

| Instruction | Name for SHA256 |
|---|---|
| Mint MEMO | `process_mint` |
| Create Profile | `create_profile` |
| Update Profile | `update_profile` |
| Delete Profile | `delete_profile` |
| Send Chat Message | `send_memo_to_group` |
| Create Chat Group | `create_chat_group` |
| Burn for Chat Group | `burn_tokens_for_group` |
| Create Forum Post | `create_post` |
| Burn for Forum Post | `burn_for_post` |
| Mint for Forum Post | `mint_for_post` |
| Create Blog | `create_blog` |
| Update Blog | `update_blog` |
| Burn for Blog | `burn_for_blog` |
| Mint for Blog | `mint_for_blog` |
| Create Project | `create_project` |
| Update Project | `update_project` |
| Burn for Project | `burn_for_project` |
| Init Burn Stats | `initialize_user_global_burn_stats` |

All computed as: `SHA256("global:<name>").slice(0, 8)`

---

## Quick Reference: Common Queries

| Task | web3.js Method | Key Param |
|---|---|---|
| Check XNT balance | `connection.getBalance()` | pubkey |
| Check MEMO balance | `connection.getTokenAccountsByOwner()` | owner + mint filter |
| MEMO total supply | `connection.getTokenSupply()` | MEMO mint address |
| User profile | `connection.getAccountInfo()` | Profile PDA |
| Forum post | `connection.getAccountInfo()` | Post PDA |
| Blog | `connection.getAccountInfo()` | Blog PDA |
| Project | `connection.getAccountInfo()` | Project PDA |
| Chat group | `connection.getAccountInfo()` | Chat Group PDA |
| Token holders | `connection.getProgramAccounts()` | Token-2022 + memcmp mint |
| Top burners | `connection.getProgramAccounts()` | Burn program + dataSize:65 |
| Tx history | `connection.getSignaturesForAddress()` | address + limit |
| Tx details | `connection.getTransaction()` | signature |
| Chat messages | `getSignaturesForAddress()` on group PDA | parse memo field |
| Health check | `connection.getVersion()` | (none) |

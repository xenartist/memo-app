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

## Architecture: How Mint & Burn Works

### Contract Architecture (CPI Call Graph)

```
                    ┌───────────────────────────────────────┐
                    │          MEMO Protocol                 │
                    │       Contract Architecture            │
                    └───────────────────────────────────────┘

 ┌────────────────────────────────────────────────────────────────────────────┐
 │                         Upper-layer Programs                               │
 │       (business logic + state management, CPI-calls core for mint/burn)    │
 │                                                                            │
 │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐   │
 │  │  PROFILE   │ │   CHAT    │ │   FORUM   │ │   BLOG    │ │  PROJECT  │   │
 │  │  2BY8v..   │ │  Hni4q..  │ │  6gzhG..  │ │  3EKdp..  │ │  6Vavo..  │   │
 │  │            │ │           │ │           │ │           │ │           │   │
 │  │ create     │ │ send_memo │ │create_post│ │create_blog│ │create_proj│   │
 │  │ update     │ │create_grp │ │ burn_for  │ │update_blog│ │update_proj│   │
 │  │ delete     │ │ burn_for  │ │ mint_for  │ │ burn_for  │ │ burn_for  │   │
 │  │            │ │           │ │           │ │ mint_for  │ │           │   │
 │  │  CPI:      │ │  CPI:     │ │  CPI:     │ │  CPI:     │ │  CPI:     │   │
 │  │  BURN only │ │ BURN+MINT │ │ BURN+MINT │ │ BURN+MINT │ │ BURN only │   │
 │  └─────┬──────┘ └──┬────┬──┘ └──┬────┬──┘ └──┬────┬──┘ └─────┬──────┘   │
 │        │           │    │       │    │       │    │           │           │
 └────────┼───────────┼────┼───────┼────┼───────┼────┼───────────┼───────────┘
          │           │    │       │    │       │    │           │
          │ CPI       │CPI │CPI   │CPI │CPI   │CPI │CPI       │ CPI
          ▼           ▼    ▼      ▼    ▼      ▼    ▼           ▼
 ┌────────────────────────────────────────────────────────────────────────────┐
 │                          Core Programs                                     │
 │                (atomic token operations, no business logic)                 │
 │                                                                            │
 │  ┌─────────────────────────────┐   ┌─────────────────────────────┐        │
 │  │       BURN_PROGRAM          │   │       MINT_PROGRAM           │        │
 │  │       2sb3g...              │   │       8iq6z...               │        │
 │  │                             │   │                              │        │
 │  │  process_burn()             │   │  process_mint()              │        │
 │  │  initialize_burn_stats()    │   │  process_mint_to()           │        │
 │  │                             │   │                              │        │
 │  │  Validates:                 │   │  Validates:                  │        │
 │  │  • SPL Memo at index 0     │   │  • SPL Memo at index 0      │        │
 │  │  • Borsh+Base64 memo       │   │  • Memo length 69-800       │        │
 │  │  • Burn amount match       │   │  • Supply cap (10T max)     │        │
 │  │  • User burn stats update  │   │  • Dynamic mint amount      │        │
 │  └──────────────┬──────────────┘   └──────────────┬───────────────┘        │
 │                 │                                  │                        │
 │                 ▼                                  ▼                        │
 │  ┌─────────────────────────────────────────────────────────────┐           │
 │  │              Token-2022 Program (SPL)                        │           │
 │  │       MEMO Token: memoX1sJsBY6od7CfQ58XooRALwnocAZen        │           │
 │  │                  burn() / mint_to()                          │           │
 │  └─────────────────────────────────────────────────────────────┘           │
 └────────────────────────────────────────────────────────────────────────────┘

 CPI Direction Summary:
 ──────────────────────
 PROFILE  ──CPI──▶ BURN_PROGRAM ──▶ Token-2022.burn()
 CHAT     ──CPI──▶ BURN_PROGRAM ──▶ Token-2022.burn()
 CHAT     ──CPI──▶ MINT_PROGRAM ──▶ Token-2022.mint_to()
 FORUM    ──CPI──▶ BURN_PROGRAM ──▶ Token-2022.burn()
 FORUM    ──CPI──▶ MINT_PROGRAM ──▶ Token-2022.mint_to()
 BLOG     ──CPI──▶ BURN_PROGRAM ──▶ Token-2022.burn()
 BLOG     ──CPI──▶ MINT_PROGRAM ──▶ Token-2022.mint_to()
 PROJECT  ──CPI──▶ BURN_PROGRAM ──▶ Token-2022.burn()
```

### Two Paths to Mint/Burn

| Path | When to use | What happens |
|---|---|---|
| **Direct → Core Program** (`MINT_PROGRAM` / `BURN_PROGRAM`) | Pure mint/burn with no business logic. Minimal dependencies, lowest complexity, easiest to debug. | Your transaction calls `process_mint` or `process_burn` directly. You build the SPL Memo instruction yourself and place it at index 0. |
| **Via Upper-layer Program** (Forum / Chat / Project / Blog / Profile) | Mint/burn **+ business logic** (e.g. posting requires burn, chatting triggers mint, profile creation burns tokens). | Your transaction calls the upper-layer program (e.g. `create_post`). That program internally **CPI-calls** the core `BURN_PROGRAM` or `MINT_PROGRAM` to execute the actual token operation. |

> **One-liner**: Core = atomic token operation; Upper-layer = atomic operation + business semantics + state changes.

### The Role of SPL Memo

SPL Memo is **not** the mechanism that performs mint/burn — it is a **data payload carrier**:

- All MEMO Protocol contracts require an SPL Memo instruction at **index 0** of the transaction
- The memo carries Borsh+Base64 encoded structured data (BurnMemo with payload)
- The contract reads and validates the memo content from the Instructions Sysvar
- The actual mint/burn is executed by the core program (directly or via CPI)

```
Transaction layout (upper-layer example):
┌─────────────────────────────────────────────────────────┐
│ Index 0: SPL Memo instruction (Borsh+Base64 payload)    │  ← data carrier
│ Index 1: Upper-layer instruction (e.g. create_post)     │  ← business logic
│          └─ CPI → BURN_PROGRAM.process_burn()           │  ← actual burn
│ Index 2+: ComputeBudget instructions                    │  ← CU limit/price
└─────────────────────────────────────────────────────────┘

Transaction layout (direct core example):
┌─────────────────────────────────────────────────────────┐
│ Index 0: SPL Memo instruction (Borsh+Base64 payload)    │  ← data carrier
│ Index 1: BURN_PROGRAM.process_burn(amount)              │  ← direct burn
│ Index 2+: ComputeBudget instructions                    │  ← CU limit/price
└─────────────────────────────────────────────────────────┘
```

### Verifying CPI Calls (Debug Method)

To confirm an upper-layer program actually CPI'd into the core program, inspect the transaction logs:

```javascript
const tx = await connection.getTransaction(signature, {
  maxSupportedTransactionVersion: 0,
  commitment: 'confirmed',
});

// Method 1: Check logMessages for CPI invoke chain
const logs = tx.meta.logMessages;
// Look for patterns like:
//   "Program <CHAT_PROGRAM> invoke [1]"       ← your instruction
//   "Program <BURN_PROGRAM> invoke [2]"        ← CPI call to core
//   "Program <BURN_PROGRAM> success"           ← core burn succeeded
//   "Program <CHAT_PROGRAM> success"           ← upper-layer succeeded

const cpiToBurn = logs.some(log =>
  log.includes('2sb3gz5Cmr2g1ia5si2rmCZqPACxgaZXEmiS5k6Htcvh invoke [2]')
);
const cpiToMint = logs.some(log =>
  log.includes('8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX invoke [2]')
);
console.log('CPI to BURN_PROGRAM:', cpiToBurn);
console.log('CPI to MINT_PROGRAM:', cpiToMint);

// Method 2: Check innerInstructions for programId
const innerIxs = tx.meta.innerInstructions || [];
for (const inner of innerIxs) {
  for (const ix of inner.instructions) {
    const programId = tx.transaction.message.accountKeys[ix.programIdIndex];
    if (programId.toString() === '2sb3gz5Cmr2g1ia5si2rmCZqPACxgaZXEmiS5k6Htcvh') {
      console.log('Inner instruction calls BURN_PROGRAM');
    }
    if (programId.toString() === '8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX') {
      console.log('Inner instruction calls MINT_PROGRAM');
    }
  }
}
```

The `invoke [2]` depth confirms it's a CPI call (depth 1 = top-level, depth 2 = CPI from depth 1).

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

> **Note**: `getProgramAccounts` with `encoding:'jsonParsed'` on Token-2022 may fail on X1 RPC
> due to extension parsing incompatibilities — some accounts fall back to raw `base64` format
> instead of `parsed` objects. Use `getTokenLargestAccounts` instead — it is more stable and performant.

```javascript
// Recommended: getTokenLargestAccounts (stable on X1, returns top 20)
const result = await connection.getTokenLargestAccounts(MEMO_MINT);
const topHolders = result.value.map(item => ({
  tokenAccount: item.address.toBase58(),
  amount: item.amount,       // raw amount in lamports
  uiAmount: item.uiAmount,   // human-readable amount
  decimals: item.decimals,
}));

// Optional: resolve owner addresses for each token account
for (const holder of topHolders) {
  const info = await connection.getParsedAccountInfo(new PublicKey(holder.tokenAccount));
  if (info.value?.data?.parsed?.info?.owner) {
    holder.owner = info.value.data.parsed.info.owner;
  }
}
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

### Encode Forum Post Reply — Burn (burn_for_post)

Burn tokens to reply to an existing forum post. Anyone can reply to any post.

> **Note**: Unlike `create_post` which has separate title/content/image fields,
> `burn_for_post` only has a single `message` field (max 512 chars).
> If you need structured content in a reply, encode it as formatted text in the message.

```javascript
function encodePostBurn(user, postId, message) {
  return Buffer.concat([
    encodeBorshU8(1),                        // version
    encodeBorshString('forum'),              // category
    encodeBorshString('burn_for_post'),      // operation
    encodeBorshString(user),                 // user (Base58 pubkey)
    encodeBorshU64(postId),                  // post_id (target post)
    encodeBorshString(message),              // reply message (max 512 chars)
  ]);
}

// Example: reply to post #5 with a burn of 2 MEMO
const payload = encodePostBurn(keypair.publicKey.toBase58(), 5, 'Great post! Burning 2 MEMO to support.');
const memoBase64 = encodeBurnMemo(2_000_000, payload); // burn 2 MEMO
```

### Encode Forum Post Reply — Mint (mint_for_post)

Mint tokens to reply to an existing forum post. Anyone can reply to any post.

> **Note**: `mint_for_post` also only has a `message` field (same as burn_for_post).
> The memo uses `BurnMemo` wrapper with `burn_amount = 0` (since no tokens are burned).

```javascript
function encodePostMint(user, postId, message) {
  return Buffer.concat([
    encodeBorshU8(1),                        // version
    encodeBorshString('forum'),              // category
    encodeBorshString('mint_for_post'),      // operation
    encodeBorshString(user),                 // user (Base58 pubkey)
    encodeBorshU64(postId),                  // post_id (target post)
    encodeBorshString(message),              // reply message (max 512 chars)
  ]);
}

// Example: reply to post #5 and earn a mint reward
const payload = encodePostMint(keypair.publicKey.toBase58(), 5, 'Interesting perspective, thanks for sharing!');
const memoBase64 = encodeBurnMemo(0, payload); // burn_amount = 0 for mint operations
```

### Encode Blog Burn (burn_for_blog)

Burn tokens for a blog entry. **Creator-only** — only the blog owner can burn for their blog.

```javascript
function encodeBlogBurn(burner, message) {
  return Buffer.concat([
    encodeBorshU8(1),                        // version
    encodeBorshString('blog'),               // category
    encodeBorshString('burn_for_blog'),      // operation
    encodeBorshString(burner),               // burner (Base58 pubkey, must be blog creator)
    encodeBorshString(message),              // message (max 696 chars)
  ]);
}

// Example: blog creator burns 10 MEMO with an update message
const payload = encodeBlogBurn(keypair.publicKey.toBase58(), 'New article published! Burning to boost visibility.');
const memoBase64 = encodeBurnMemo(10_000_000, payload); // burn 10 MEMO
```

### Encode Blog Mint (mint_for_blog)

Mint tokens for a blog entry. **Creator-only** — only the blog owner can mint for their blog.

```javascript
function encodeBlogMint(minter, message) {
  return Buffer.concat([
    encodeBorshU8(1),                        // version
    encodeBorshString('blog'),               // category
    encodeBorshString('mint_for_blog'),      // operation
    encodeBorshString(minter),               // minter (Base58 pubkey, must be blog creator)
    encodeBorshString(message),              // message (max 696 chars)
  ]);
}

// Example: blog creator mints with a log message
const payload = encodeBlogMint(keypair.publicKey.toBase58(), 'Daily check-in on my tech blog.');
const memoBase64 = encodeBurnMemo(0, payload); // burn_amount = 0 for mint operations
```

### Encode Project Burn (burn_for_project)

Burn tokens for a project. **Creator-only** — only the project owner can burn for their project.

```javascript
function encodeProjectBurn(projectId, burner, message) {
  return Buffer.concat([
    encodeBorshU8(1),                        // version
    encodeBorshString('project'),            // category
    encodeBorshString('burn_for_project'),   // operation
    encodeBorshU64(projectId),               // project_id (target project)
    encodeBorshString(burner),               // burner (Base58 pubkey, must be project creator)
    encodeBorshString(message),              // message (max 696 chars)
  ]);
}

// Example: project creator burns 100 MEMO for project #3
const payload = encodeProjectBurn(3, keypair.publicKey.toBase58(), 'Milestone 1 completed!');
const memoBase64 = encodeBurnMemo(100_000_000, payload); // burn 100 MEMO
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

### Decoding Pipeline (Step-by-Step)

The full decoding pipeline for a compressed pixel art string like
`c:32x32:bY3RCcBADEKHERwgs2QBwf1nqGnvaKHnVx7PoH1Ku2mVYNOoxchlgPGIeZiteNX02ZunFY/pY/H36+VZuBeTfG7+Rxc=`

```
Step 1: Split by ':' (max 3 parts)
  → fmt = "c"
  → size = "32x32" → width=32, height=32
  → data = "bY3RCcBADEKHERwgs2QBwf1nqGnvaKHnVx7PoH1Ku2mVYNOoxchlgPGIeZiteNX02ZunFY/pY/H36+VZuBeTfG7+Rxc="

Step 2: Base64 decode 'data' → compressed bytes (raw binary, NOT pixels!)

Step 3: Deflate decompress → "safe string" (171 chars for 32×32)
  Example output: "#####A)B&Lk..." (171 ASCII characters)

Step 4: Decode each safe-string character → 6-bit value (0-63)
  e.g. '#' → 0 (000000), '$' → 1 (000001), '%' → 2 (000010), ...

Step 5: Expand each 6-bit value → 6 individual pixel bits
  e.g. value 42 = 101010 → pixels: [1,0,1,0,1,0]

Step 6: Flatten all bits into a 1D array (row-major order)
  Total bits = width × height = 1024 for 32×32
  Pixel index: row = floor(i / width), col = i % width

Step 7: Reshape into 2D grid
  grid[row][col] = 1 (black/filled) or 0 (white/empty)
```

**IMPORTANT**: The Base64 data in `c:` format is NOT raw pixels! It is Base64-encoded **Deflate-compressed** safe-string characters. You MUST decompress first, then decode the safe-string character-by-character.

For `n:` (normal) format, skip Steps 2-3 — the data after the second `:` IS the safe string directly.

### Complete Rendering Example (JavaScript)

```javascript
import { inflateRawSync } from 'zlib';

// Full pipeline: image string → ASCII art
function renderPixelArt(imageString) {
  // Step 1: Parse format
  const parts = imageString.split(':');
  let fmt, width, height, data;

  if (parts.length === 3) {
    fmt = parts[0];
    const [w, h] = parts[1].split('x').map(Number);
    width = w; height = h;
    data = parts[2];
  } else if (parts.length === 2) {
    fmt = parts[0];
    data = parts[1];
    width = 32; height = 32;
  } else {
    return 'Invalid format';
  }

  // Step 2-3: Get safe string
  let safeString;
  if (fmt === 'c') {
    // Compressed: Base64 → bytes → Deflate decompress → safe string
    const compressed = Buffer.from(data, 'base64');
    safeString = inflateRawSync(compressed).toString('ascii');
  } else if (fmt === 'n') {
    // Normal: data IS the safe string
    safeString = data;
  } else {
    return 'Unknown format: ' + fmt;
  }

  // Step 4-5: Decode safe string → pixel bits
  const pixels = [];
  for (const c of safeString) {
    const value = decodePixelChar(c);
    if (value === null) return 'Invalid character: ' + c;
    // Extract 6 bits, MSB first
    for (let i = 5; i >= 0; i--) {
      pixels.push((value >> i) & 1);
    }
  }

  // Step 6-7: Reshape to 2D grid and render
  const lines = [];
  for (let y = 0; y < height; y++) {
    let line = '';
    for (let x = 0; x < width; x++) {
      const idx = y * width + x;
      const bit = idx < pixels.length ? pixels[idx] : 0;
      line += bit ? '█' : '░';  // 1=black(filled), 0=white(empty)
    }
    lines.push(line);
  }

  return lines.join('\n');
}

// Render as raw binary matrix (0s and 1s)
function renderPixelArtBinary(imageString) {
  // ... same Steps 1-5 as above to get pixels[] ...
  // (use decodePixelArt() from previous section to get { width, height, grid })

  const result = decodePixelArt(imageString);
  if (!result) return 'Decode failed';

  const lines = [];
  for (let y = 0; y < result.height; y++) {
    let line = '';
    for (let x = 0; x < result.width; x++) {
      line += result.grid[y][x] ? '1' : '0';
    }
    lines.push(line);
  }
  return lines.join('\n');
  // Output for 32x32:
  // 10101001001010100101010010101010
  // 01010010100101010010101001010101
  // ... (32 lines of 32 chars each)
}

// Usage:
const img = 'c:32x32:bY3RCcBADEKHERwgs2QBwf1nqGnvaKHnVx7PoH1Ku2mVYNOoxchlgPGIeZiteNX02ZunFY/pY/H36+VZuBeTfG7+Rxc=';
console.log(renderPixelArt(img));     // Visual: █░█░...
console.log(renderPixelArtBinary(img)); // Binary: 1010...
```

### Rendering Output Formats

| Format | Black pixel (1) | White pixel (0) | Use case |
|---|---|---|---|
| Binary | `1` | `0` | Data processing, debugging |
| Block chars | `█` | `░` | Terminal / monospace display |
| Telegram | `⬛` | `⬜` | Chat bots (emoji blocks) |
| HTML Canvas | `fillRect(x,y,s,s)` | (skip) | Web rendering |
| SVG | `<rect fill="black"/>` | (skip) | Scalable display |

### Key Facts for Agents

- **1 bit per pixel**: each pixel is binary — black (1) or white (0), no colors, no palette
- **6 bits per character**: each safe-string character encodes exactly 6 pixels
- **32×32 = 1024 pixels = 171 characters** (ceil(1024/6) = 171)
- **Compressed format `c:`** = Base64(Deflate(safe_string)), NOT Base64(raw_pixels)
- **Normal format `n:`** = safe_string directly (no compression)
- **Row-major order**: pixel index `i` → row `floor(i/width)`, col `i%width`
- **Do NOT Base64-decode `c:` data and treat bytes as pixels** — you will get gibberish

---

## Write Operations (Transaction Building)

All write operations follow this pattern:

1. **Build instructions** (SPL Memo instruction MUST be at index 0 when required)
2. **Get latest blockhash**
3. **Simulate transaction** (via raw `fetch` JSON-RPC, NOT `connection.simulateTransaction()`) to estimate actual compute units consumed
4. **Set precise compute budget** based on simulation result (with ~1% buffer)
5. **Sign and send** the final transaction

> **STRONGLY RECOMMENDED on X1**: Always simulate first to determine the exact CU needed, then set `ComputeUnitLimit` accordingly. This saves CU and reduces transaction fees. Do NOT hardcode a large CU value like 400,000 — use simulation to get the real number and add a ~1% buffer.
>
> **CRITICAL**: Use raw `fetch` to call `simulateTransaction` RPC — do NOT use `connection.simulateTransaction()` from `@solana/web3.js`. The SDK wrapper causes `"Invalid arguments"` errors on X1 RPC due to internal serialization differences. See the `buildAndSendTransaction` helper below for the correct implementation.

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

### Transaction Sending Helper (with Simulation-based CU)

> **CRITICAL: X1 RPC Compatibility**
> Do NOT use `connection.simulateTransaction()` from `@solana/web3.js` on X1.
> The SDK wrapper may serialize parameters differently from what X1 RPC expects,
> causing `"Invalid arguments"` errors. Always use **raw `fetch` JSON-RPC calls**
> for simulation, as shown below. This matches how the official MEMO frontend works.

```javascript
const RPC_URL = 'https://rpc.mainnet.x1.xyz';

/**
 * Send a raw JSON-RPC request to X1.
 * Used for simulateTransaction because connection.simulateTransaction() is
 * incompatible with X1 RPC (causes "Invalid arguments").
 */
async function rpcRequest(method, params) {
  const res = await fetch(RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
  });
  const json = await res.json();
  if (json.error) {
    throw new Error(`RPC error ${json.error.code}: ${json.error.message}`);
  }
  return json.result;
}

async function buildAndSendTransaction(connection, keypair, instructions, cuBufferMultiplier = 1.01) {
  const { blockhash } = await connection.getLatestBlockhash('confirmed');

  // ── Step 1: Simulate to estimate compute units ──
  // Build an UNSIGNED simulation transaction with a high CU limit
  const simTx = new Transaction();
  for (const ix of instructions) {
    simTx.add(ix);
  }
  // Use a large CU limit for simulation so it won't fail due to CU exhaustion
  simTx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }));
  simTx.recentBlockhash = blockhash;
  simTx.feePayer = keypair.publicKey;

  // Serialize unsigned transaction to base64 (same as Rust: bincode → base64)
  const simTxBytes = simTx.serialize({
    requireAllSignatures: false,    // allow unsigned
    verifySignatures: false,
  });
  const simTxBase64 = simTxBytes.toString('base64');

  // Call simulateTransaction via raw fetch (NOT connection.simulateTransaction)
  // This is critical for X1 compatibility — the web3.js wrapper causes "Invalid arguments"
  const simResult = await rpcRequest('simulateTransaction', [
    simTxBase64,
    {
      encoding: 'base64',
      commitment: 'confirmed',
      sigVerify: false,              // skip signature verification (unsigned tx)
      replaceRecentBlockhash: true,  // auto-replace blockhash for freshness
    },
  ]);

  if (simResult.value.err) {
    throw new Error(
      `Simulation failed: ${JSON.stringify(simResult.value.err)}\n` +
      `Logs: ${(simResult.value.logs || []).join('\n')}`
    );
  }

  const simulatedCU = simResult.value.unitsConsumed;
  const finalCU = Math.ceil(simulatedCU * cuBufferMultiplier);
  console.log(`Simulated: ${simulatedCU} CU → Final: ${finalCU} CU (${cuBufferMultiplier}x buffer)`);

  // ── Step 2: Build final transaction with precise CU limit ──
  const tx = new Transaction();
  for (const ix of instructions) {
    tx.add(ix);
  }
  tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: finalCU }));
  // Optional: set priority fee (uncomment if needed)
  // tx.add(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 1000 }));

  tx.recentBlockhash = blockhash;
  tx.feePayer = keypair.publicKey;

  const signature = await sendAndConfirmTransaction(connection, tx, [keypair], {
    commitment: 'confirmed',
    maxRetries: 3,
  });
  return signature;
}
```

**Why simulate first?**
- Default CU limit on X1 is 400,000 per instruction. If you hardcode a large value like 400,000, you pay fees for unused CU.
- Simulation runs the transaction without signing or committing, returning `unitsConsumed` — the exact CU the transaction will use.
- We use `sigVerify: false` and `replaceRecentBlockhash: true` so you don't need to sign the simulation transaction.
- Add a small buffer (~1.01x) because actual execution may vary slightly from simulation.

**Why raw `fetch` instead of `connection.simulateTransaction()`?**
- `@solana/web3.js`'s `connection.simulateTransaction()` wrapper internally serializes the transaction and parameters in a way that X1 RPC may reject with `"Invalid arguments"`.
- The official MEMO frontend (Rust/WASM) uses raw JSON-RPC `fetch` calls for simulation — this is the **proven, working approach** on X1.
- By calling `simulateTransaction` via raw `fetch`, you have full control over the encoding (`base64`), `sigVerify`, and `replaceRecentBlockhash` parameters, exactly matching what the RPC endpoint expects.
- `connection.getLatestBlockhash()`, `sendAndConfirmTransaction()`, and other read/write SDK methods work fine on X1 — the issue is specifically with `simulateTransaction`.

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
  return await buildAndSendTransaction(connection, keypair, [ix]);
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

  return await buildAndSendTransaction(connection, keypair, [ix]);
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

  return await buildAndSendTransaction(connection, keypair, [ix]);
}
```

### W10b. Direct Core Burn (process_burn)

Directly calls the core `BURN_PROGRAM.process_burn()` to burn MEMO tokens **without** going through any upper-layer program. This is the simplest burn path — no business logic, no state changes, just pure token destruction with a Borsh+Base64 memo.

Use this when you want maximum control, lowest complexity, and easiest debugging. The upper-layer burn operations (W4, W5, W8, W9, W12, W16, W20) internally CPI-call this same `process_burn` instruction.

**Requirements:**
- Burn stats must be initialized first (see W10)
- Amount must be ≥ 1 token (1,000,000 units) and a whole number of tokens
- SPL Memo instruction MUST be at index 0 (Borsh+Base64 encoded `BurnMemo`)
- The `burn_amount` in the memo MUST exactly match the `amount` argument

```javascript
async function directCoreBurn(connection, keypair, burnAmountTokens, payload = Buffer.alloc(0)) {
  const user = keypair.publicKey;
  const burnAmountLamports = BigInt(burnAmountTokens) * BigInt(1_000_000);

  // Derive PDAs
  const { ata: userAta } = await ensureATA(connection, user, MEMO_MINT, user);
  const [burnStatsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );

  // Build BurnMemo (Borsh-serialized → Base64-encoded)
  // Structure: version (u8) + burn_amount (u64) + payload (Vec<u8>)
  const burnMemo = Buffer.concat([
    encodeBorshU8(1),                            // version = 1
    encodeBorshU64(Number(burnAmountLamports)),   // burn_amount in lamports
    encodeBorshVecU8(payload),                    // application payload (can be empty)
  ]);
  const memoBase64 = burnMemo.toString('base64');

  const instructions = [];

  // 1. SPL Memo instruction (MUST be at index 0)
  //    Contains Borsh+Base64 encoded BurnMemo data
  instructions.push(createMemoInstruction(memoBase64, user));

  // 2. Core burn instruction
  const ixData = Buffer.concat([
    anchorDiscriminator('process_burn'),
    encodeBorshU64(Number(burnAmountLamports)),   // amount argument (must match memo)
  ]);
  instructions.push(new TransactionInstruction({
    keys: [
      { pubkey: user,              isSigner: true,  isWritable: true  },  // user (signer)
      { pubkey: MEMO_MINT,         isSigner: false, isWritable: true  },  // mint
      { pubkey: userAta,           isSigner: false, isWritable: true  },  // token_account
      { pubkey: burnStatsPda,      isSigner: false, isWritable: true  },  // user_global_burn_stats
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },  // token_program
      { pubkey: INSTRUCTIONS_SYSVAR,   isSigner: false, isWritable: false },  // instructions sysvar
    ],
    programId: BURN_PROGRAM,
    data: ixData,
  }));

  return await buildAndSendTransaction(connection, keypair, instructions);
}

// Usage: burn 10 MEMO tokens with empty payload
const sig = await directCoreBurn(connection, keypair, 10);

// Usage: burn 5 MEMO tokens with a custom Borsh payload (e.g. from encodeChatGroupBurn)
const customPayload = encodeChatGroupBurn(groupId, burnerPubkey, 'Support message');
const sig2 = await directCoreBurn(connection, keypair, 5, customPayload);
```

**Key differences from upper-layer burns (W12, W16, W20):**
- Calls `BURN_PROGRAM` directly — no CPI, no intermediate program
- No business-logic side effects (no post/blog/project state updates)
- You construct the `BurnMemo` yourself and place it at index 0
- The `amount` argument and the `burn_amount` field in the memo **must match exactly**, or the contract rejects with `BurnAmountMismatch`
- Burn stats (`user_global_burn_stats`) are still updated (total_burned, burn_count, last_burn_time)

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

  // Build reply payload using helper (see "Encode Forum Post Reply — Burn")
  const payload = encodePostBurn(user.toBase58(), postId, message);
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

// Usage: reply to post #42 with a burn of 5 MEMO
const sig = await burnForPost(connection, keypair, 42, 'Great analysis! Burning to support.', 5);
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

  // Build reply payload using helper (see "Encode Forum Post Reply — Mint")
  // mint operations: BurnMemo wrapper with burn_amount = 0
  const payload = encodePostMint(user.toBase58(), postId, message);
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

// Usage: reply to post #42 and earn a mint reward
const sig = await mintForPost(connection, keypair, 42, 'Thanks for the insight, really helpful!');
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

  // Build payload using helper (see "Encode Blog Burn")
  const payload = encodeBlogBurn(user.toBase58(), message);
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

// Usage: blog creator burns 5 MEMO with a message
const sig = await burnForBlog(connection, keypair, keypair.publicKey.toBase58(), 'New article: Intro to X1', 5);
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

  // Build payload using helper (see "Encode Blog Mint")
  const payload = encodeBlogMint(user.toBase58(), message);
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

// Usage: blog creator mints with a message
const sig = await mintForBlog(connection, keypair, keypair.publicKey.toBase58(), 'Weekly update on my blog.');
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

  // Build payload using helper (see "Encode Project Burn")
  const payload = encodeProjectBurn(projectId, user.toBase58(), message);
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

// Usage: project creator burns 420 MEMO for project #3
const sig = await burnForProject(connection, keypair, 3, 'Milestone 2 shipped!', 420);
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

## Pre-flight Checklist (Balance & Prerequisites)

Before executing any write operation, verify these prerequisites. Skipping these checks is the #1 cause of transaction failures.

### Universal Checks (ALL write operations)

```javascript
// 1. Check XNT (SOL) balance for gas fees
const balance = await connection.getBalance(keypair.publicKey);
const MIN_GAS_LAMPORTS = 10_000_000; // 0.01 XNT — safe minimum for most txs
if (balance < MIN_GAS_LAMPORTS) {
  throw new Error(`Insufficient XNT for gas: ${balance / 1e9} XNT (need ≥ 0.01)`);
}

// 2. Check MEMO token balance (for burn operations)
async function getMemoBalance(connection, owner) {
  const ata = await getAssociatedTokenAddress(MEMO_MINT, owner, false, TOKEN_2022_PROGRAM_ID);
  const info = await connection.getAccountInfo(ata);
  if (!info) return { exists: false, balance: 0 };
  // Token account data: offset 64 = amount (u64 LE)
  const amount = Number(info.data.readBigUInt64LE(64));
  return { exists: true, balance: amount };
}

// 3. Check if burn stats are initialized (required for ALL burn operations)
async function isBurnStatsInitialized(connection, user) {
  const [statsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('user_global_burn_stats'), user.toBuffer()], BURN_PROGRAM
  );
  const info = await connection.getAccountInfo(statsPda);
  return info !== null;
}
```

### Per-Operation Checklist

| Operation | XNT Gas | MEMO ATA | MEMO Balance | Burn Stats | Other |
|---|---|---|---|---|---|
| **W1. Mint MEMO** | ✅ | auto-created | — | — | Memo text 69-800 bytes |
| **W2. Transfer XNT** | ✅ | — | — | — | Sufficient XNT for transfer + gas |
| **W3. Transfer MEMO** | ✅ | ✅ | ✅ amount | — | Recipient ATA may need creation |
| **W4. Create Profile** | ✅ | ✅ | ✅ ≥ 420 | ✅ init first | — |
| **W5. Update Profile** | ✅ | ✅ | ✅ ≥ 420 | ✅ | Profile must exist |
| **W6. Delete Profile** | ✅ | — | — | — | Profile must exist |
| **W7. Send Chat Message** | ✅ | auto-created | — | — | Group must exist |
| **W8. Create Chat Group** | ✅ | ✅ | ✅ ≥ 42,069 | ✅ init first | — |
| **W9. Burn for Chat Group** | ✅ | ✅ | ✅ ≥ 1 | ✅ | Group must exist |
| **W10. Init Burn Stats** | ✅ | — | — | creates it | One-time per user |
| **W10b. Direct Core Burn** | ✅ | ✅ | ✅ ≥ 1 | ✅ | — |
| **W11. Create Forum Post** | ✅ | ✅ | ✅ ≥ 1 | ✅ init first | — |
| **W12. Burn for Forum Post** | ✅ | ✅ | ✅ ≥ 1 | ✅ | Post must exist |
| **W13. Mint for Forum Post** | ✅ | auto-created | — | — | Post must exist |
| **W14. Create Blog** | ✅ | ✅ | ✅ ≥ 1 | ✅ init first | One blog per user |
| **W15. Update Blog** | ✅ | ✅ | ✅ ≥ 1 | ✅ | Blog must exist, creator only |
| **W16. Burn for Blog** | ✅ | ✅ | ✅ ≥ 1 | ✅ | Blog must exist, creator only |
| **W17. Mint for Blog** | ✅ | auto-created | — | — | Blog must exist, creator only |
| **W18. Create Project** | ✅ | ✅ | ✅ ≥ 42,069 | ✅ init first | — |
| **W19. Update Project** | ✅ | ✅ | ✅ ≥ 42,069 | ✅ | Project must exist, creator only |
| **W20. Burn for Project** | ✅ | ✅ | ✅ ≥ 420 | ✅ | Project must exist, creator only |

**Legend**: ✅ = required, — = not needed, "auto-created" = `ensureATA` handles it, "init first" = call W10 if not yet initialized

### Recommended Pre-flight Sequence

```javascript
// Run this before any burn operation:
async function preflight(connection, keypair, requiredMemoTokens = 0) {
  const user = keypair.publicKey;

  // 1. Gas check
  const xntBalance = await connection.getBalance(user);
  if (xntBalance < 10_000_000) {
    throw new Error(`Need XNT for gas. Balance: ${xntBalance / 1e9} XNT`);
  }

  // 2. MEMO balance check (for burn operations)
  if (requiredMemoTokens > 0) {
    const { exists, balance } = await getMemoBalance(connection, user);
    const requiredLamports = requiredMemoTokens * 1_000_000;
    if (!exists) {
      throw new Error('MEMO token account does not exist. Mint some MEMO first (W1).');
    }
    if (balance < requiredLamports) {
      throw new Error(
        `Insufficient MEMO: have ${balance / 1e6}, need ${requiredMemoTokens}. Mint more (W1).`
      );
    }
  }

  // 3. Burn stats check (for burn operations)
  if (requiredMemoTokens > 0) {
    const initialized = await isBurnStatsInitialized(connection, user);
    if (!initialized) {
      console.log('Burn stats not initialized. Initializing now...');
      await initializeBurnStats(connection, keypair); // W10
    }
  }

  console.log('Pre-flight checks passed.');
}

// Usage:
await preflight(connection, keypair, 420); // Need 420 MEMO for profile creation
await createProfile(connection, keypair, 'alice', 'n:32x32:...', 'Hello!', 420);
```

---

## Error Code Quick Reference

### How to Extract Errors

```javascript
try {
  const sig = await buildAndSendTransaction(connection, keypair, instructions);
} catch (error) {
  // Method 1: From simulation logs
  if (error.logs) {
    const errorLog = error.logs.find(log => log.includes('Error Message:'));
    if (errorLog) console.error('Program error:', errorLog);
  }

  // Method 2: From error message
  // The error string often contains "custom program error: 0xNNNN"
  const match = error.message?.match(/custom program error: (0x[0-9a-fA-F]+)/);
  if (match) {
    const code = parseInt(match[1], 16);
    console.error(`Error code: ${code} (${match[1]})`);
  }

  // Method 3: RPC-level error (before reaching program)
  if (error.message?.includes('Invalid arguments')) {
    console.error('RPC rejected the request. Use raw fetch for simulateTransaction.');
  }
}
```

### Anchor Error Codes (All Programs)

Anchor reserves codes 6000+. Each program starts from 6000 for its own errors.

#### Core Burn Program (BURN_PROGRAM)

| Code | Hex | Name | Cause | Fix |
|---|---|---|---|---|
| 6000 | 0x1770 | MemoRequired | No SPL Memo at index 0 | Add memo instruction as first instruction |
| 6001 | 0x1771 | InvalidMemoFormat | Memo is not valid Borsh+Base64 | Check encoding: Borsh serialize → Base64 encode |
| 6002 | 0x1772 | UnsupportedMemoVersion | `version` ≠ 1 | Set `version: 1` in BurnMemo |
| 6003 | 0x1773 | BurnAmountTooSmall | Amount < 1 token (< 1,000,000) | Increase burn amount to ≥ 1 token |
| 6004 | 0x1774 | BurnAmountTooLarge | Amount > 1T tokens per tx | Reduce burn amount |
| 6005 | 0x1775 | InvalidBurnAmount | Amount not multiple of 1,000,000 | Use whole token amounts only |
| 6006 | 0x1776 | InvalidTokenAccount | Token account wrong mint | Check ATA is for MEMO_MINT |
| 6007 | 0x1777 | UnauthorizedMint | Mint ≠ MEMO_MINT | Use correct MEMO_MINT address |
| 6008 | 0x1778 | UnauthorizedTokenAccount | ATA owner ≠ signer | Use signer's own ATA |
| 6009 | 0x1779 | BurnAmountMismatch | Memo `burn_amount` ≠ instruction amount | Ensure they match exactly |
| 6010 | 0x177A | MemoTooShort | Memo < 69 bytes | Ensure encoded memo ≥ 69 bytes |
| 6011 | 0x177B | MemoTooLong | Memo > 800 bytes | Shorten payload content |
| 6012 | 0x177C | PayloadTooLong | Payload > 787 bytes | Reduce payload size |
| 6013 | 0x177D | UnauthorizedUser | Burn stats user ≠ signer | Use correct burn stats PDA |

#### Core Mint Program (MINT_PROGRAM)

| Code | Hex | Name | Cause | Fix |
|---|---|---|---|---|
| 6000 | 0x1770 | MemoRequired | No SPL Memo at index 0 | Add memo instruction first |
| 6001 | 0x1771 | InvalidMemoFormat | Memo contains null bytes | Remove null bytes |
| 6002 | 0x1772 | MemoTooShort | Memo < 69 bytes | Ensure memo ≥ 69 bytes |
| 6003 | 0x1773 | MemoTooLong | Memo > 800 bytes | Shorten memo |
| 6004 | 0x1774 | InvalidTokenAccount | Wrong mint | Use MEMO_MINT ATA |
| 6005 | 0x1775 | UnauthorizedMint | Wrong mint address | Use correct MEMO_MINT |
| 6006 | 0x1776 | UnauthorizedTokenAccount | ATA owner ≠ signer | Use signer's own ATA |
| 6007 | 0x1777 | InvalidMintAuthority | PDA mismatch | Derive from MINT_PROGRAM with seed `"mint_authority"` |
| 6008 | 0x1778 | SupplyLimitReached | Supply ≥ 10T tokens | Max supply cap hit (unlikely) |

#### Forum Program (FORUM_PROGRAM)

| Code | Hex | Name | Cause | Fix |
|---|---|---|---|---|
| 6000 | 0x1770 | MemoRequired | No SPL Memo at index 0 | Add memo instruction first |
| 6001 | 0x1771 | InvalidMemoFormat | Bad Borsh+Base64 | Check encoding |
| 6002 | 0x1772 | UnsupportedMemoVersion | `version` ≠ 1 | Set `version: 1` |
| 6003 | 0x1773 | BurnAmountTooSmall | Burn < 1 MEMO for post | Increase burn amount |
| 6007 | 0x1777 | InvalidPostDataFormat | `PostCreationData` Borsh decode failed | Check field order and types |
| 6008 | 0x1778 | InvalidPostBurnDataFormat | `PostBurnData` Borsh decode failed | Check field order: version, category, operation, user, post_id, message |
| 6009 | 0x1779 | InvalidPostMintDataFormat | `PostMintData` Borsh decode failed | Check field order |
| 6010 | 0x177A | InvalidMintMemoFormat | Mint memo has `burn_amount` ≠ 0 | Set `burn_amount: 0` for mint ops |
| 6015 | 0x177F | PostIdMismatch | Expected post ID ≠ actual | Re-read global counter |
| 6019 | 0x1783 | CreatorPubkeyMismatch | Memo creator ≠ signer | Use signer's pubkey in payload |
| 6026 | 0x178A | ReplyMessageTooLong | Reply > 512 chars | Shorten message |

#### Solana Runtime Errors (Pre-program)

| Error | Cause | Fix |
|---|---|---|
| `"Invalid arguments"` | `connection.simulateTransaction()` on X1 | Use raw `fetch` JSON-RPC (see `rpcRequest` helper) |
| `InsufficientFunds` (0x1) | Not enough XNT for gas fees | Fund wallet with XNT |
| `InsufficientFunds` on token transfer | Not enough MEMO to burn | Mint more MEMO first (W1) |
| `AccountNotFound` | PDA/ATA does not exist | Create it first (init burn stats, ensure ATA, etc.) |
| `AccountAlreadyInUse` | Trying to init already-existing account | Check with `getAccountInfo` before init |
| `TransactionTooLarge` | Transaction > 1232 bytes | Reduce payload / split into multiple txs |
| `BlockhashNotFound` | Blockhash expired | Retry with fresh `getLatestBlockhash()` |

---

## ComputeBudget Instruction Placement

> **CRITICAL RULE**: MEMO Protocol contracts require `SPL Memo at index 0`. This means `ComputeBudgetProgram` instructions **cannot** be placed at the front of the instruction list.

### Placement Rules

```
✅ CORRECT instruction order:
  Index 0: SPL Memo instruction          ← REQUIRED at index 0 by contract
  Index 1: Program instruction (mint/burn/create_post/etc.)
  Index 2: ComputeBudgetProgram.setComputeUnitLimit(...)
  Index 3: ComputeBudgetProgram.setComputeUnitPrice(...)   (optional)

❌ WRONG — will cause MemoRequired error:
  Index 0: ComputeBudgetProgram.setComputeUnitLimit(...)   ← NOT a memo!
  Index 1: SPL Memo instruction
  Index 2: Program instruction
```

**Why this works**: Solana runtime processes `ComputeBudgetProgram` instructions **before** executing any other instruction, regardless of their position in the instruction list. So even at index 2/3, they take effect before the program runs.

**Exceptions**: Operations that don't require SPL Memo (e.g., `initializeBurnStats`, `deleteProfile`, `transferXNT`) can place `ComputeBudget` instructions anywhere.

The `buildAndSendTransaction` helper handles this automatically — it always appends `ComputeBudget` instructions **after** your base instructions.

---

## Runnable Minimum Example

Copy-paste this script to verify your setup works. It performs a core mint (W1) — the simplest write operation (no MEMO balance needed, no burn stats needed).

```javascript
// ── minimum_test.mjs ──
// Usage: node minimum_test.mjs <base58_private_key>
// Requires: npm install @solana/web3.js @solana/spl-token

import { Connection, Keypair, Transaction, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { getAssociatedTokenAddress, createAssociatedTokenAccountInstruction, TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import { createHash } from 'crypto';

// ── Constants ──
const RPC_URL   = 'https://rpc.mainnet.x1.xyz';
const MEMO_MINT = new PublicKey('memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick');
const MINT_PROGRAM    = new PublicKey('8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX');
const SPL_MEMO_PROGRAM = new PublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr');
const INSTRUCTIONS_SYSVAR = new PublicKey('Sysvar1nstructions1111111111111111111111111');

// ── Helpers ──
function anchorDiscriminator(name) {
  return createHash('sha256').update(`global:${name}`).digest().subarray(0, 8);
}

async function rpcRequest(method, params) {
  const res = await fetch(RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
  });
  const json = await res.json();
  if (json.error) throw new Error(`RPC error ${json.error.code}: ${json.error.message}`);
  return json.result;
}

// ── Main ──
async function main() {
  const secret = Uint8Array.from(JSON.parse(process.argv[2] || '[]'));
  if (secret.length !== 64) {
    // Try base58 decode if not JSON array
    const bs58 = await import('bs58');
    const key = bs58.default.decode(process.argv[2]);
    var keypair = Keypair.fromSecretKey(key);
  } else {
    var keypair = Keypair.fromSecretKey(secret);
  }
  const connection = new Connection(RPC_URL, 'confirmed');
  const user = keypair.publicKey;
  console.log('Wallet:', user.toBase58());

  // Step 1: Check gas balance
  const balance = await connection.getBalance(user);
  console.log('XNT balance:', balance / 1e9);
  if (balance < 5_000_000) throw new Error('Need at least 0.005 XNT for gas');

  // Step 2: Build memo text (must be 69-800 bytes)
  const memoText = 'Hello MEMO Protocol! ' + 'x'.repeat(50); // pad to ≥ 69 bytes

  // Step 3: Derive PDAs and ATA
  const [mintAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from('mint_authority')], MINT_PROGRAM
  );
  const userAta = await getAssociatedTokenAddress(MEMO_MINT, user, false, TOKEN_2022_PROGRAM_ID);

  // Step 4: Build instructions
  const instructions = [];

  // Index 0: SPL Memo
  instructions.push(new TransactionInstruction({
    keys: [{ pubkey: user, isSigner: true, isWritable: true }],
    programId: SPL_MEMO_PROGRAM,
    data: Buffer.from(memoText, 'utf-8'),
  }));

  // Create ATA if needed
  const ataInfo = await connection.getAccountInfo(userAta);
  if (!ataInfo) {
    instructions.push(
      createAssociatedTokenAccountInstruction(user, userAta, user, MEMO_MINT, TOKEN_2022_PROGRAM_ID)
    );
  }

  // Mint instruction
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

  // Step 5: Simulate (raw fetch)
  const { blockhash } = await connection.getLatestBlockhash('confirmed');
  const { ComputeBudgetProgram } = await import('@solana/web3.js');
  const simTx = new Transaction();
  instructions.forEach(ix => simTx.add(ix));
  simTx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }));
  simTx.recentBlockhash = blockhash;
  simTx.feePayer = user;

  const simBytes = simTx.serialize({ requireAllSignatures: false, verifySignatures: false });
  const simResult = await rpcRequest('simulateTransaction', [
    simBytes.toString('base64'),
    { encoding: 'base64', commitment: 'confirmed', sigVerify: false, replaceRecentBlockhash: true },
  ]);

  if (simResult.value.err) {
    console.error('Simulation FAILED:', JSON.stringify(simResult.value.err));
    console.error('Logs:', simResult.value.logs?.join('\n'));
    process.exit(1);
  }

  const cu = simResult.value.unitsConsumed;
  console.log(`Simulation OK: ${cu} CU consumed`);

  // Step 6: Build final tx with precise CU
  const finalTx = new Transaction();
  instructions.forEach(ix => finalTx.add(ix));
  finalTx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: Math.ceil(cu * 1.01) }));
  finalTx.recentBlockhash = blockhash;
  finalTx.feePayer = user;

  // Step 7: Sign and send
  const { sendAndConfirmTransaction } = await import('@solana/web3.js');
  const sig = await sendAndConfirmTransaction(connection, finalTx, [keypair], {
    commitment: 'confirmed', maxRetries: 3,
  });
  console.log('SUCCESS! Signature:', sig);
  console.log(`Explorer: https://explorer.x1.xyz/tx/${sig}`);
}

main().catch(err => { console.error(err); process.exit(1); });
```

**Test sequence (recommended order):**
1. `minimum_test.mjs` (core mint) — verifies wallet, gas, RPC, simulation all work
2. `initializeBurnStats` (W10) — one-time prerequisite
3. `directCoreBurn` (W10b) with 1 MEMO — verifies burn path
4. `burnForPost` / `mintForPost` (W12/W13) — verifies upper-layer CPI

---

## Security & Production Best Practices

### Wallet Safety

- **NEVER use your main wallet for automated agents.** Create a dedicated hot wallet with limited funds.
- Set an XNT upper limit on the hot wallet (e.g., 1 XNT = ~1000 transactions).
- Monitor the hot wallet balance and alert if it drops unexpectedly.

### Data Immutability

- **Everything on-chain is permanent and public.** The `message`, `title`, `content`, and `image` fields in memos **cannot be deleted or edited** after submission.
- Never include private keys, passwords, API tokens, PII, or sensitive data in any memo field.
- Consider content moderation policies before auto-posting with agents.

### Rate Limiting & Retry Strategy

```javascript
// Recommended retry wrapper for agents
async function withRetry(fn, maxRetries = 3, baseDelayMs = 2000) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error) {
      const isRetryable =
        error.message?.includes('BlockhashNotFound') ||
        error.message?.includes('blockhash') ||
        error.message?.includes('Too many requests') ||
        error.message?.includes('429');

      if (!isRetryable || attempt === maxRetries) throw error;

      const delay = baseDelayMs * Math.pow(2, attempt - 1); // exponential backoff
      console.log(`Attempt ${attempt} failed (${error.message}). Retrying in ${delay}ms...`);
      await new Promise(r => setTimeout(r, delay));
    }
  }
}

// Usage:
const sig = await withRetry(() => buildAndSendTransaction(connection, keypair, instructions));
```

### Idempotency

- **`initializeBurnStats`** is idempotent — always check with `getAccountInfo` first (the W10 helper does this).
- **`createProfile` / `createBlog` / `createProject`** are NOT idempotent — calling twice will fail (`AccountAlreadyInUse`). Always check if the PDA already exists.
- **`mint_for_post` / `burn_for_post`** are inherently non-idempotent (each call creates a new reply). Use a nonce or dedup key in your agent logic if needed.

### Agent-Specific Recommendations

- Add a **cooldown** between auto-posts (e.g., 10s minimum) to avoid rate limits.
- Log every transaction signature for auditability.
- Use `preflight()` helper before every write operation.
- Set a daily budget limit (total MEMO burned) for unattended agents.

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
}
```

### X1 RPC Known Compatibility Issues

| Issue | Symptom | Solution |
|---|---|---|
| `connection.simulateTransaction()` fails | `"Invalid arguments"` before reaching program logic | Use raw `fetch` JSON-RPC call instead (see `rpcRequest` + `buildAndSendTransaction` helper) |
| `getProgramAccounts` with `jsonParsed` for Token-2022 | Returns unparseable data or Buffer | Use `getTokenLargestAccounts` for token holders instead |
| Transaction encoding | Some SDK wrappers use incompatible serialization | Always serialize with `tx.serialize({ requireAllSignatures: false, verifySignatures: false })` and encode as base64 manually |

---

## Instruction Discriminator Quick Reference

| Instruction | Name for SHA256 |
|---|---|
| Mint MEMO | `process_mint` |
| Direct Core Burn | `process_burn` |
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

| Task | Method | Key Param |
|---|---|---|
| Check XNT balance | `connection.getBalance()` | pubkey |
| Check MEMO balance | `connection.getTokenAccountsByOwner()` | owner + mint filter |
| MEMO total supply | `connection.getTokenSupply()` | MEMO mint address |
| User profile | `connection.getAccountInfo()` | Profile PDA |
| Forum post | `connection.getAccountInfo()` | Post PDA |
| Blog | `connection.getAccountInfo()` | Blog PDA |
| Project | `connection.getAccountInfo()` | Project PDA |
| Chat group | `connection.getAccountInfo()` | Chat Group PDA |
| Token holders (top 20) | `connection.getTokenLargestAccounts()` | MEMO mint address |
| Top burners | `connection.getProgramAccounts()` | Burn program + dataSize:65 |
| Tx history | `connection.getSignaturesForAddress()` | address + limit |
| Tx details | `connection.getTransaction()` | signature |
| Chat messages | `getSignaturesForAddress()` on group PDA | parse memo field |
| Simulate transaction | raw `fetch` → `simulateTransaction` RPC | base64 tx + `sigVerify:false` |
| Health check | `connection.getVersion()` | (none) |

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::Path;
use vtagent::tree_sitter::{LanguageSupport, TreeSitterAnalyzer};

/// Benchmark tree-sitter parsing performance
fn benchmark_tree_sitter_parsing(c: &mut Criterion) {
    let mut analyzer = TreeSitterAnalyzer::new().unwrap();

    let mut group = c.benchmark_group("tree_sitter");

    // Benchmark Rust parsing
    let rust_code = create_large_rust_file();
    group.bench_function("parse_rust_large", |b| {
        b.iter(|| {
            let _tree = analyzer.parse(black_box(&rust_code), LanguageSupport::Rust);
        });
    });

    // Benchmark Python parsing
    let python_code = create_large_python_file();
    group.bench_function("parse_python_large", |b| {
        b.iter(|| {
            let _tree = analyzer.parse(black_box(&python_code), LanguageSupport::Python);
        });
    });

    // Benchmark JavaScript parsing
    let js_code = create_large_js_file();
    group.bench_function("parse_js_large", |b| {
        b.iter(|| {
            let _tree = analyzer.parse(black_box(&js_code), LanguageSupport::JavaScript);
        });
    });

    // Benchmark small file parsing
    let small_rust = "fn main() { println!(\"Hello\"); }";
    group.bench_function("parse_rust_small", |b| {
        b.iter(|| {
            let _tree = analyzer.parse(black_box(small_rust), LanguageSupport::Rust);
        });
    });

    // Benchmark language detection
    group.bench_function("detect_language", |b| {
        b.iter(|| {
            let _lang = analyzer.detect_language_from_content(black_box(&rust_code));
        });
    });

    // Benchmark file analysis
    group.bench_function("analyze_file", |b| {
        b.iter(|| {
            let _analysis = analyzer
                .analyze_file_with_tree_sitter(Path::new("benchmark.rs"), black_box(&rust_code));
        });
    });

    group.finish();
}

/// Benchmark symbol extraction performance
fn benchmark_symbol_extraction(c: &mut Criterion) {
    let mut analyzer = TreeSitterAnalyzer::new().unwrap();

    let mut group = c.benchmark_group("symbol_extraction");

    let rust_code = create_complex_rust_file();

    group.bench_function("extract_symbols_rust", |b| {
        let tree = analyzer.parse(&rust_code, LanguageSupport::Rust).unwrap();
        b.iter(|| {
            let _symbols = analyzer.extract_symbols(
                black_box(&tree),
                black_box(&rust_code),
                LanguageSupport::Rust,
            );
        });
    });

    group.bench_function("extract_dependencies_rust", |b| {
        let tree = analyzer.parse(&rust_code, LanguageSupport::Rust).unwrap();
        b.iter(|| {
            let _deps = analyzer.extract_dependencies(black_box(&tree), LanguageSupport::Rust);
        });
    });

    group.bench_function("calculate_metrics_rust", |b| {
        let tree = analyzer.parse(&rust_code, LanguageSupport::Rust).unwrap();
        b.iter(|| {
            let _metrics = analyzer.calculate_metrics(black_box(&tree), black_box(&rust_code));
        });
    });

    group.finish();
}

fn create_large_rust_file() -> String {
    let mut code = String::from(
        r#"
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub profile: UserProfile,
    pub settings: UserSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserProfile {
    pub first_name: String,
    pub last_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub location: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserSettings {
    pub theme: String,
    pub notifications: NotificationSettings,
    pub privacy: PrivacySettings,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NotificationSettings {
    pub email_notifications: bool,
    pub push_notifications: bool,
    pub sms_notifications: bool,
    pub marketing_emails: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivacySettings {
    pub profile_visibility: Visibility,
    pub activity_visibility: Visibility,
    pub show_online_status: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Visibility {
    Public,
    Friends,
    Private,
}

pub struct UserService {
    users: Arc<Mutex<HashMap<u64, User>>>,
    next_id: Arc<Mutex<u64>>,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub async fn create_user(&self, username: String, email: String) -> Result<User, String> {
        if username.is_empty() || email.is_empty() {
            return Err("Username and email cannot be empty".to_string());
        }

        let mut next_id = self.next_id.lock().await;
        let user_id = *next_id;
        *next_id += 1;

        let profile = UserProfile {
            first_name: "Unknown".to_string(),
            last_name: "User".to_string(),
            bio: None,
            avatar_url: None,
            location: None,
        };

        let settings = UserSettings {
            theme: "default".to_string(),
            notifications: NotificationSettings {
                email_notifications: true,
                push_notifications: true,
                sms_notifications: false,
                marketing_emails: false,
            },
            privacy: PrivacySettings {
                profile_visibility: Visibility::Public,
                activity_visibility: Visibility::Friends,
                show_online_status: true,
            },
        };

        let user = User {
            id: user_id,
            username,
            email,
            profile,
            settings,
        };

        let mut users = self.users.lock().await;
        users.insert(user_id, user.clone());

        Ok(user)
    }

    pub async fn get_user(&self, user_id: u64) -> Option<User> {
        let users = self.users.lock().await;
        users.get(&user_id).cloned()
    }

    pub async fn update_user(&self, user_id: u64, updates: UserUpdate) -> Result<User, String> {
        let mut users = self.users.lock().await;

        if let Some(user) = users.get_mut(&user_id) {
            if let Some(first_name) = updates.first_name {
                user.profile.first_name = first_name;
            }
            if let Some(last_name) = updates.last_name {
                user.profile.last_name = last_name;
            }
            if let Some(bio) = updates.bio {
                user.profile.bio = bio;
            }
            if let Some(location) = updates.location {
                user.profile.location = location;
            }

            Ok(user.clone())
        } else {
            Err("User not found".to_string())
        }
    }

    pub async fn delete_user(&self, user_id: u64) -> Result<(), String> {
        let mut users = self.users.lock().await;

        if users.remove(&user_id).is_some() {
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    pub async fn list_users(&self) -> Vec<User> {
        let users = self.users.lock().await;
        users.values().cloned().collect()
    }

    pub async fn search_users(&self, query: &str) -> Vec<User> {
        let users = self.users.lock().await;
        users
            .values()
            .filter(|user| {
                user.username.to_lowercase().contains(&query.to_lowercase()) ||
                user.email.to_lowercase().contains(&query.to_lowercase()) ||
                format!("{} {}", user.profile.first_name, user.profile.last_name)
                    .to_lowercase()
                    .contains(&query.to_lowercase())
            })
            .cloned()
            .collect()
    }
}

#[derive(Debug)]
pub struct UserUpdate {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<Option<String>>,
    pub location: Option<Option<String>>,
}

impl UserUpdate {
    pub fn new() -> Self {
        Self {
            first_name: None,
            last_name: None,
            bio: None,
            location: None,
        }
    }

    pub fn first_name(mut self, first_name: String) -> Self {
        self.first_name = Some(first_name);
        self
    }

    pub fn last_name(mut self, last_name: String) -> Self {
        self.last_name = Some(last_name);
        self
    }

    pub fn bio(mut self, bio: Option<String>) -> Self {
        self.bio = Some(bio);
        self
    }

    pub fn location(mut self, location: Option<String>) -> Self {
        self.location = Some(location);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_user() {
        let service = UserService::new();

        let user = service
            .create_user("testuser".to_string(), "test@example.com".to_string())
            .await
            .unwrap();

        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.id, 1);
    }

    #[tokio::test]
    async fn test_get_user() {
        let service = UserService::new();
        let created_user = service
            .create_user("testuser".to_string(), "test@example.com".to_string())
            .await
            .unwrap();

        let retrieved_user = service.get_user(created_user.id).await.unwrap();
        assert_eq!(retrieved_user.id, created_user.id);
        assert_eq!(retrieved_user.username, created_user.username);
    }

    #[tokio::test]
    async fn test_update_user() {
        let service = UserService::new();
        let user = service
            .create_user("testuser".to_string(), "test@example.com".to_string())
            .await
            .unwrap();

        let update = UserUpdate::new()
            .first_name("John".to_string())
            .last_name("Doe".to_string())
            .bio(Some("Software Developer".to_string()));

        let updated_user = service.update_user(user.id, update).await.unwrap();
        assert_eq!(updated_user.profile.first_name, "John");
        assert_eq!(updated_user.profile.last_name, "Doe");
        assert_eq!(updated_user.profile.bio, Some("Software Developer".to_string()));
    }

    #[tokio::test]
    async fn test_delete_user() {
        let service = UserService::new();
        let user = service
            .create_user("testuser".to_string(), "test@example.com".to_string())
            .await
            .unwrap();

        service.delete_user(user.id).await.unwrap();

        let retrieved_user = service.get_user(user.id).await;
        assert!(retrieved_user.is_none());
    }

    #[tokio::test]
    async fn test_list_users() {
        let service = UserService::new();

        service
            .create_user("user1".to_string(), "user1@example.com".to_string())
            .await
            .unwrap();
        service
            .create_user("user2".to_string(), "user2@example.com".to_string())
            .await
            .unwrap();

        let users = service.list_users().await;
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_search_users() {
        let service = UserService::new();

        service
            .create_user("johndoe".to_string(), "john@example.com".to_string())
            .await
            .unwrap();
        service
            .create_user("janedoe".to_string(), "jane@example.com".to_string())
            .await
            .unwrap();

        let search_results = service.search_users("doe").await;
        assert_eq!(search_results.len(), 2);

        let john_search = service.search_users("john").await;
        assert_eq!(john_search.len(), 1);
        assert_eq!(john_search[0].username, "johndoe");
    }
}
"#,
    );

    // Add repeated code to make it larger for benchmarking
    for i in 0..20 {
        code.push_str(&format!(
            r#"
// Additional module {} for benchmarking
pub mod module_{} {{
    pub fn helper_function_{}() -> i32 {{
        let mut result = 0;
        for j in 0..100 {{
            result += j * {};
        }}
        result
    }}

    pub struct HelperStruct{} {{
        pub value: i32,
    }}

    impl HelperStruct{} {{
        pub fn new() -> Self {{
            Self {{
                value: helper_function_{}(),
            }}
        }}

        pub fn compute(&self) -> i32 {{
            self.value * 2
        }}
    }}
}}
"#,
            i, i, i, i, i, i, i
        ));
    }

    code
}

fn create_large_python_file() -> String {
    let mut code = String::from(
        r#"
import asyncio
import json
from typing import List, Dict, Optional, Any
from dataclasses import dataclass
from datetime import datetime, timedelta
import logging

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

@dataclass
class User:
    id: int
    username: str
    email: str
    first_name: str
    last_name: str
    is_active: bool = True
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None

    def __post_init__(self):
        if self.created_at is None:
            self.created_at = datetime.now()
        if self.updated_at is None:
            self.updated_at = datetime.now()

    def full_name(self) -> str:
        return f"{self.first_name} {self.last_name}"

    def is_recently_active(self) -> bool:
        if self.updated_at is None:
            return False
        return datetime.now() - self.updated_at < timedelta(days=7)

    def to_dict(self) -> Dict[str, Any]:
        return {
            'id': self.id,
            'username': self.username,
            'email': self.email,
            'first_name': self.first_name,
            'last_name': self.last_name,
            'is_active': self.is_active,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
        }

class UserService:
    def __init__(self):
        self.users: Dict[int, User] = {}
        self.next_id = 1

    def create_user(self, username: str, email: str, first_name: str, last_name: str) -> User:
        if not username or not email:
            raise ValueError("Username and email are required")

        user = User(
            id=self.next_id,
            username=username,
            email=email,
            first_name=first_name,
            last_name=last_name
        )

        self.users[self.next_id] = user
        self.next_id += 1

        logger.info(f"Created user: {user.username}")
        return user

    def get_user(self, user_id: int) -> Optional[User]:
        return self.users.get(user_id)

    def update_user(self, user_id: int, **kwargs) -> Optional[User]:
        user = self.users.get(user_id)
        if not user:
            return None

        for key, value in kwargs.items():
            if hasattr(user, key):
                setattr(user, key, value)

        user.updated_at = datetime.now()
        logger.info(f"Updated user {user_id}")
        return user

    def delete_user(self, user_id: int) -> bool:
        if user_id in self.users:
            del self.users[user_id]
            logger.info(f"Deleted user {user_id}")
            return True
        return False

    def list_users(self, active_only: bool = False) -> List[User]:
        users = list(self.users.values())
        if active_only:
            users = [u for u in users if u.is_active]
        return users

    def search_users(self, query: str) -> List[User]:
        query = query.lower()
        return [
            user for user in self.users.values()
            if query in user.username.lower() or
               query in user.email.lower() or
               query in user.first_name.lower() or
               query in user.last_name.lower()
        ]

    def deactivate_inactive_users(self, days: int = 90) -> int:
        cutoff_date = datetime.now() - timedelta(days=days)
        deactivated_count = 0

        for user in self.users.values():
            if user.updated_at and user.updated_at < cutoff_date and user.is_active:
                user.is_active = False
                user.updated_at = datetime.now()
                deactivated_count += 1

        logger.info(f"Deactivated {deactivated_count} inactive users")
        return deactivated_count

async def async_create_users(service: UserService, count: int) -> List[User]:
    tasks = []
    for i in range(count):
        task = asyncio.create_task(
            asyncio.to_thread(
                service.create_user,
                f"user_{i}",
                f"user_{i}@example.com",
                f"First{i}",
                f"Last{i}"
            )
        )
        tasks.append(task)

    users = await asyncio.gather(*tasks)
    return users

def main():
    service = UserService()

    # Create some test users
    for i in range(10):
        service.create_user(
            f"user_{i}",
            f"user_{i}@example.com",
            f"First{i}",
            f"Last{i}"
        )

    # List all users
    users = service.list_users()
    print(f"Created {len(users)} users")

    # Search for users
    search_results = service.search_users("user_5")
    print(f"Found {len(search_results)} users matching 'user_5'")

    # Update a user
    updated = service.update_user(1, first_name="UpdatedFirst")
    if updated:
        print(f"Updated user: {updated.full_name()}")

    # Deactivate inactive users
    deactivated = service.deactivate_inactive_users(30)
    print(f"Deactivated {deactivated} inactive users")

if __name__ == "__main__":
    main()
"#,
    );

    // Add more code to make it larger
    for i in 0..10 {
        code.push_str(&format!(
            r#"

# Additional module {0} for benchmarking
class Module{0}:
    def __init__(self):
        self.value = {0}
        self.data = []

    def process_data(self, data: List[int]) -> List[int]:
        result = []
        for item in data:
            processed = item * self.value + {0}
            result.append(processed)
            self.data.append(processed)
        return result

    def get_statistics(self) -> Dict[str, float]:
        if not self.data:
            return {{'min': 0.0, 'max': 0.0, 'avg': 0.0}}

        return {{
            'min': min(self.data),
            'max': max(self.data),
            'avg': sum(self.data) / len(self.data)
        }}

    async def async_process(self, items: List[int]) -> List[int]:
        import asyncio
        tasks = []
        for item in items:
            task = asyncio.create_task(self._process_item(item))
            tasks.append(task)

        return await asyncio.gather(*tasks)

    async def _process_item(self, item: int) -> int:
        await asyncio.sleep(0.001)  # Simulate async work
        return item * self.value

def create_module_{0}() -> Module{0}:
    return Module{0}()

# Global instance for testing
module_{0}_instance = create_module_{0}()
"#,
            i
        ));
    }

    code
}

fn create_large_js_file() -> String {
    let mut code = String::from(
        r#"
const fs = require('fs').promises;
const path = require('path');
const crypto = require('crypto');

class User {
    constructor(id, username, email, firstName, lastName) {
        this.id = id;
        this.username = username;
        this.email = email;
        this.firstName = firstName;
        this.lastName = lastName;
        this.isActive = true;
        this.createdAt = new Date();
        this.updatedAt = new Date();
        this.profile = {
            bio: null,
            avatar: null,
            location: null
        };
        this.settings = {
            theme: 'default',
            notifications: {
                email: true,
                push: true,
                sms: false
            }
        };
    }

    get fullName() {
        return `${this.firstName} ${this.lastName}`;
    }

    updateProfile(updates) {
        Object.assign(this.profile, updates);
        this.updatedAt = new Date();
    }

    deactivate() {
        this.isActive = false;
        this.updatedAt = new Date();
    }

    toJSON() {
        return {
            id: this.id,
            username: this.username,
            email: this.email,
            firstName: this.firstName,
            lastName: this.lastName,
            isActive: this.isActive,
            createdAt: this.createdAt.toISOString(),
            updatedAt: this.updatedAt.toISOString(),
            profile: this.profile,
            settings: this.settings
        };
    }
}

class UserService {
    constructor() {
        this.users = new Map();
        this.nextId = 1;
    }

    createUser(username, email, firstName, lastName) {
        if (!username || !email) {
            throw new Error('Username and email are required');
        }

        const user = new User(this.nextId, username, email, firstName, lastName);
        this.users.set(this.nextId, user);
        this.nextId++;

        console.log(`Created user: ${user.username}`);
        return user;
    }

    getUser(userId) {
        return this.users.get(userId) || null;
    }

    updateUser(userId, updates) {
        const user = this.users.get(userId);
        if (!user) {
            return null;
        }

        Object.assign(user, updates);
        user.updatedAt = new Date();

        console.log(`Updated user ${userId}`);
        return user;
    }

    deleteUser(userId) {
        if (this.users.has(userId)) {
            this.users.delete(userId);
            console.log(`Deleted user ${userId}`);
            return true;
        }
        return false;
    }

    listUsers(activeOnly = false) {
        const users = Array.from(this.users.values());
        return activeOnly ? users.filter(u => u.isActive) : users;
    }

    searchUsers(query) {
        const lowercaseQuery = query.toLowerCase();
        return Array.from(this.users.values()).filter(user =>
            user.username.toLowerCase().includes(lowercaseQuery) ||
            user.email.toLowerCase().includes(lowercaseQuery) ||
            user.firstName.toLowerCase().includes(lowercaseQuery) ||
            user.lastName.toLowerCase().includes(lowercaseQuery)
        );
    }

    async saveToFile(filePath) {
        const usersData = Array.from(this.users.values()).map(u => u.toJSON());
        await fs.writeFile(filePath, JSON.stringify(usersData, null, 2));
    }

    async loadFromFile(filePath) {
        try {
            const data = await fs.readFile(filePath, 'utf8');
            const usersData = JSON.parse(data);

            this.users.clear();
            for (const userData of usersData) {
                const user = new User(
                    userData.id,
                    userData.username,
                    userData.email,
                    userData.firstName,
                    userData.lastName
                );
                user.isActive = userData.isActive;
                user.createdAt = new Date(userData.createdAt);
                user.updatedAt = new Date(userData.updatedAt);
                user.profile = userData.profile;
                user.settings = userData.settings;

                this.users.set(user.id, user);
            }

            console.log(`Loaded ${usersData.length} users from file`);
        } catch (error) {
            console.error('Error loading users from file:', error.message);
            throw error;
        }
    }
}

// Utility functions
function generateRandomString(length) {
    return crypto.randomBytes(length).toString('hex');
}

function validateEmail(email) {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
}

function hashPassword(password) {
    return crypto.createHash('sha256').update(password).digest('hex');
}

async function processUsersBatch(users, batchSize = 10) {
    const results = [];

    for (let i = 0; i < users.length; i += batchSize) {
        const batch = users.slice(i, i + batchSize);
        const batchPromises = batch.map(async (user) => {
            // Simulate async processing
            await new Promise(resolve => setTimeout(resolve, 10));
            return user.toJSON();
        });

        const batchResults = await Promise.all(batchPromises);
        results.push(...batchResults);
    }

    return results;
}

// Export for use in other modules
module.exports = {
    User,
    UserService,
    generateRandomString,
    validateEmail,
    hashPassword,
    processUsersBatch
};
"#,
    );

    // Add more code to make it larger
    for i in 0..15 {
        code.push_str(&format!(
            r#"

// Additional module {0} for benchmarking
class Module{0} {{
    constructor() {{
        this.id = {0};
        this.data = [];
        this.cache = new Map();
    }}

    processItem(item) {{
        if (this.cache.has(item)) {{
            return this.cache.get(item);
        }}

        const result = item * this.id + {0};
        this.cache.set(item, result);
        this.data.push(result);
        return result;
    }}

    processBatch(items) {{
        return items.map(item => this.processItem(item));
    }}

    async processBatchAsync(items) {{
        const promises = items.map(async (item) => {{
            await new Promise(resolve => setTimeout(resolve, 1));
            return this.processItem(item);
        }});

        return await Promise.all(promises);
    }}

    getStatistics() {{
        if (this.data.length === 0) {{
            return {{ min: 0, max: 0, avg: 0 }};
        }}

        const min = Math.min(...this.data);
        const max = Math.max(...this.data);
        const avg = this.data.reduce((sum, val) => sum + val, 0) / this.data.length;

        return {{ min, max, avg }};
    }}

    clearCache() {{
        this.cache.clear();
        this.data = [];
    }}
}}

function createModule{0}() {{
    return new Module{0}();
}}

// Global instances for testing
const module{0}Instance = createModule{0}();
global.module{0}Instance = module{0}Instance;
"#,
            i
        ));
    }

    code
}

fn create_complex_rust_file() -> String {
    let mut code = String::new();

    // Create a complex Rust file with many symbols
    for i in 0..50 {
        code.push_str(&format!(
            r#"
pub mod module_{} {{
    use std::collections::HashMap;
    use serde::{{Serialize, Deserialize}};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Struct{}(pub i32, pub String);

    #[derive(Debug)]
    pub enum Enum{} {{
        Variant1,
        Variant2(i32),
        Variant3 {{ field: String }},
    }}

    pub trait Trait{} {{
        fn method_{}(&self) -> i32;
        fn default_method(&self) {{ println!("Default"); }}
    }}

    impl Trait{} for Struct{} {{
        fn method_{}(&self) -> i32 {{
            self.0 + {}
        }}
    }}

    pub fn function_{}() -> i32 {{
        {}
    }}

    pub const CONST_{}: i32 = {};

    pub static STATIC_{}: i32 = {};
}}

pub use module_{}::*;
"#,
            i, i, i, i, i, i, i, i, i, i, i, i, i, i, i, i
        ));
    }

    code
}

criterion_group!(
    benches,
    benchmark_tree_sitter_parsing,
    benchmark_symbol_extraction
);
criterion_main!(benches);

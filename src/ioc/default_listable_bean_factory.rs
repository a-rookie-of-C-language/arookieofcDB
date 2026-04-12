use super::bean_definition::{BeanDefinition, BeanScope, SharedBean};
use super::bean_factory::{BeanDefinitionRegistry, BeanFactory};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock};

pub struct DefaultListableBeanFactory {
    bean_definition_map: RwLock<HashMap<String, Box<dyn BeanDefinition>>>,
    bean_definition_names: RwLock<Vec<String>>,
    singleton_objects: RwLock<HashMap<String, SharedBean>>,
    currently_in_creation: Mutex<HashSet<String>>,
    environment: RwLock<HashMap<String, String>>,
}

impl Default for DefaultListableBeanFactory {
    fn default() -> Self {
        Self {
            bean_definition_map: RwLock::new(HashMap::new()),
            bean_definition_names: RwLock::new(Vec::new()),
            singleton_objects: RwLock::new(HashMap::new()),
            currently_in_creation: Mutex::new(HashSet::new()),
            environment: RwLock::new(HashMap::new()),
        }
    }
}

impl DefaultListableBeanFactory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_environment(&mut self, env: HashMap<String, String>) {
        *self.environment.write().unwrap() = env;
    }

    pub fn get_environment(&self) -> HashMap<String, String> {
        self.environment.read().unwrap().clone()
    }

    pub fn destroy_singletons(&mut self) {
        self.singleton_objects.write().unwrap().clear();
    }
}

impl BeanDefinitionRegistry for DefaultListableBeanFactory {
    fn contains_bean_definition(&self, bean_name: &str) -> bool {
        self.bean_definition_map
            .read()
            .unwrap()
            .contains_key(bean_name)
    }

    fn get_bean_definition(&self, bean_name: &str) -> Option<&dyn BeanDefinition> {
        let bean_map = self.bean_definition_map.read().unwrap();
        bean_map.get(bean_name).map(|definition| {
            let ptr: *const dyn BeanDefinition = definition.as_ref();
            unsafe { &*ptr }
        })
    }

    fn get_bean_definition_count(&self) -> usize {
        self.bean_definition_map.read().unwrap().len()
    }

    fn get_bean_definition_names(&self) -> Vec<String> {
        self.bean_definition_names.read().unwrap().clone()
    }

    fn is_bean_name_in_use(&self, bean_name: &str) -> bool {
        self.contains_bean_definition(bean_name)
            || self.singleton_objects.read().unwrap().contains_key(bean_name)
            || self.currently_in_creation.lock().unwrap().contains(bean_name)
    }

    fn register_bean_definition(
        &mut self,
        bean_name: &str,
        bean_definition: Box<dyn BeanDefinition>,
    ) {
        self.bean_definition_map
            .write()
            .unwrap()
            .insert(bean_name.to_string(), bean_definition);
        self.bean_definition_names
            .write()
            .unwrap()
            .push(bean_name.to_string());
    }

    fn remove_bean_definition(&mut self, bean_name: &str) {
        self.bean_definition_map.write().unwrap().remove(bean_name);
        self.bean_definition_names
            .write()
            .unwrap()
            .retain(|n| n != bean_name);
    }
}

impl BeanFactory for DefaultListableBeanFactory {
    fn get_bean(&self, name: &str) -> Option<SharedBean> {
        if let Some(bean) = self.singleton_objects.read().unwrap().get(name).cloned() {
            return Some(bean);
        }
        self.do_create_bean(name)
    }

    fn is_singleton(&self, name: &str) -> bool {
        self.bean_definition_map
            .read()
            .unwrap()
            .get(name)
            .map(|definition| definition.get_scope() == BeanScope::Singleton)
            .unwrap_or_else(|| self.singleton_objects.read().unwrap().contains_key(name))
    }

    fn contains_bean(&self, name: &str) -> bool {
        self.bean_definition_map.read().unwrap().contains_key(name)
            || self.singleton_objects.read().unwrap().contains_key(name)
    }

    fn do_create_bean(&self, name: &str) -> Option<SharedBean> {
        let (dependencies, scope) = {
            let bean_map = self.bean_definition_map.read().unwrap();
            let Some(definition) = bean_map.get(name) else {
                self.currently_in_creation.lock().unwrap().remove(name);
                return None;
            };
            (definition.get_dependencies(), definition.get_scope())
        };

        if scope == BeanScope::Singleton {
            if let Some(bean) = self.singleton_objects.read().unwrap().get(name).cloned() {
                return Some(bean);
            }
        }

        {
            let mut creating = self.currently_in_creation.lock().unwrap();
            if creating.contains(name) {
                eprintln!("[ioc] circular dependency detected at bean '{}'", name);
                return None;
            }
            creating.insert(name.to_string());
        }

        for dep in &dependencies {
            if self.currently_in_creation.lock().unwrap().contains(dep) {
                eprintln!("[ioc] circular dependency detected: {} -> {}", name, dep);
                self.currently_in_creation.lock().unwrap().remove(name);
                return None;
            }
            self.do_create_bean(dep);
        }

        let deps_snapshot: HashMap<String, SharedBean> = {
            let singletons = self.singleton_objects.read().unwrap();
            dependencies
                .iter()
                .filter_map(|dep_name| {
                    singletons.get(dep_name).cloned().map(|bean| {
                        (dep_name.clone(), bean)
                    })
                })
                .collect()
        };

        let env_snapshot = self.environment.read().unwrap().clone();

        let bean = {
            let bean_map = self.bean_definition_map.read().unwrap();
            let Some(definition) = bean_map.get(name) else {
                self.currently_in_creation.lock().unwrap().remove(name);
                return None;
            };
            definition.create_instance(&deps_snapshot, &env_snapshot)
        };

        if scope == BeanScope::Singleton {
            self.singleton_objects
                .write()
                .unwrap()
                .insert(name.to_string(), bean.clone());
        }

        self.currently_in_creation.lock().unwrap().remove(name);
        Some(bean)
    }
}

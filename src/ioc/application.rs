use super::abstract_application_context::AbstractApplicationContext;
use super::application_context::ConfigurableApplicationContext;
use super::bean_definition::BeanDefinition;
use super::bean_factory::BeanDefinitionRegistry;
use super::registry::BeanRegistration;

pub struct Application;

impl Application {
    pub fn run() -> AbstractApplicationContext {
        let mut context = AbstractApplicationContext::new();

        let environment: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for registration in inventory::iter::<BeanRegistration> {
            let definition = (registration.definition)();

            if let Some((key, expected)) = definition.get_condition() {
                let actual = environment.get(key).map(|s| s.as_str()).unwrap_or("");
                if actual != expected {
                    continue;
                }
            }

            let name = definition.get_bean_class_name().to_string();
            context.register_bean_definition(&name, Box::new(definition));
        }

        context.set_environment(environment);
        context.refresh();
        context
    }
}

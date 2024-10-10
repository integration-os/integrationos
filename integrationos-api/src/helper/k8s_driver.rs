use axum::async_trait;
use integrationos_domain::{IntegrationOSError, InternalError, Unit};
use k8s_openapi::{
    api::{
        apps::v1::{Deployment, DeploymentSpec},
        core::v1::{
            Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec, Service, ServicePort,
            ServiceSpec,
        },
    },
    apimachinery::pkg::apis::meta::v1::LabelSelector,
    NamespaceResourceScope,
};
use kube::{
    api::{DeleteParams, ObjectMeta, PostParams},
    Api, Client, Resource,
};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::fmt::Debug;

#[async_trait]
pub trait K8sDriver: Send + Sync {
    async fn create_service(
        &self,
        params: ServiceSpecParams,
    ) -> Result<Service, IntegrationOSError>;
    async fn create_deployment(
        &self,
        params: DeploymentSpecParams,
    ) -> Result<Deployment, IntegrationOSError>;
    async fn delete_all(&self, namespace: &str, name: &str) -> Result<Unit, IntegrationOSError>;
    async fn coordinator(
        &self,
        service: ServiceSpecParams,
        deployment: DeploymentSpecParams,
    ) -> Result<Unit, IntegrationOSError>;
}

pub struct K8sDriverImpl {
    client: Client,
}

impl K8sDriverImpl {
    pub async fn new() -> Result<Self, IntegrationOSError> {
        let client = Client::try_default().await.map_err(|e| {
            tracing::error!("Could not connect to kubernetes: {e}");
            InternalError::io_err("Could not connect to kubernetes", None)
        })?;
        Ok(Self { client })
    }
}

#[async_trait]
impl K8sDriver for K8sDriverImpl {
    async fn create_service(
        &self,
        params: ServiceSpecParams,
    ) -> Result<Service, IntegrationOSError> {
        create_service_impl(self.client.clone(), params).await
    }

    async fn create_deployment(
        &self,
        params: DeploymentSpecParams,
    ) -> Result<Deployment, IntegrationOSError> {
        create_deployment_impl(self.client.clone(), params).await
    }

    async fn delete_all(&self, namespace: &str, name: &str) -> Result<Unit, IntegrationOSError> {
        delete_all_impl(self.client.clone(), namespace, name).await
    }

    async fn coordinator(
        &self,
        service: ServiceSpecParams,
        deployment: DeploymentSpecParams,
    ) -> Result<Unit, IntegrationOSError> {
        coordinator_impl(self.client.clone(), service, deployment).await
    }
}

#[derive(Debug, Default)]
pub struct K8sDriverLogger;

#[async_trait]
impl K8sDriver for K8sDriverLogger {
    /// Creates a new service into a given namespace
    ///
    /// # Argument:
    /// - `ServiceSpecParams` - Parameters to create the service with
    async fn create_service(
        &self,
        params: ServiceSpecParams,
    ) -> Result<Service, IntegrationOSError> {
        tracing::info!(
            "Creating k8s resource {} in namespace {}",
            params.name,
            params.namespace
        );
        Ok(Service::default())
    }

    /// Creates a new deployment into a given namespace
    ///
    /// # Argument:
    /// - `DeploymentSpecParams` - Parameters to create the deployment with
    async fn create_deployment(
        &self,
        params: DeploymentSpecParams,
    ) -> Result<Deployment, IntegrationOSError> {
        tracing::info!(
            "Creating k8s resource {} in namespace {}",
            params.name,
            params.namespace
        );
        Ok(Deployment::default())
    }

    /// Deletes all existing related resources (Deployment and Service) in a given namespace
    ///
    /// # Arguments:
    /// - `name` - Name of the deployment to delete
    /// - `namespace` - Namespace the existing deployment resides in
    async fn delete_all(&self, namespace: &str, name: &str) -> Result<Unit, IntegrationOSError> {
        tracing::info!("Deleting k8s resource {} in namespace {}", name, namespace);
        Ok(())
    }

    /// Creates a new service and deployment in a given namespace
    /// and performs cleanup in case of error
    ///
    /// # Arguments:
    /// - `service` - Parameters to create the service with
    /// - `deployment` - Parameters to create the deployment with
    ///
    /// This is the recommended way to create a new service and deployment. Due to the blind nature of
    /// of `create_service` and `create_deployment`, it is possible that you end up wasting
    /// resources.
    async fn coordinator(
        &self,
        _service: ServiceSpecParams,
        deployment: DeploymentSpecParams,
    ) -> Result<Unit, IntegrationOSError> {
        tracing::info!(
            "Creating k8s resource {} in namespace {}",
            deployment.name,
            deployment.namespace
        );
        Ok(())
    }
}

pub struct ServiceSpecParams {
    /// Ports to expose
    pub ports: Vec<ServicePort>,
    /// Selector to match against
    pub selector: BTreeMap<String, String>,
    /// Type of service: ClusterIP, NodePort, LoadBalance, ExternalName
    pub r#type: String,
    /// Labels to apply to the service
    pub labels: BTreeMap<String, String>,
    /// Annotations to apply to the service. Has to match with the deployment metadata
    pub name: String,
    /// Namespace the service should reside in
    pub namespace: String,
}

async fn create_service_impl(
    client: Client,
    params: ServiceSpecParams,
    // name: &str,
    // namespace: &str,
) -> Result<Service, IntegrationOSError> {
    // let mut labels: BTreeMap<String, String> = BTreeMap::new();
    // labels.insert("app".to_owned(), params.name);
    // labels.insert("database-type".to_owned(), "postgres".to_owned());

    let service: Service = Service {
        metadata: ObjectMeta {
            name: Some(params.name),
            labels: Some(params.labels.clone()),
            namespace: Some(params.namespace.to_owned()),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            // ServicePort {
            // name: Some("http".to_owned()),
            // port: 80,
            // target_port: Some(IntOrString::Int(8080)),
            // ..Default::default()
            // }
            ports: Some(params.ports),
            selector: Some(params.labels),
            type_: Some(params.r#type),
            ..Default::default()
        }),
        ..Default::default()
    };

    let service_api: Api<Service> = Api::namespaced(client, &params.namespace);
    service_api
        .create(&PostParams::default(), &service)
        .await
        .map_err(|e| InternalError::io_err(&format!("Could not create service: {e}"), None))
}

pub struct DeploymentSpecParams {
    /// Number of replicas to create
    pub replicas: i32,
    /// Selector to match against
    pub selector: BTreeMap<String, String>,
    /// Labels to apply to the deployment
    pub labels: BTreeMap<String, String>,
    /// Namespace the deployment should reside in
    pub namespace: String,
    /// Image to use for the deployment
    pub image: String,
    /// Environment variables to apply
    pub env: Vec<EnvVar>,
    /// Ports to expose
    pub ports: Vec<ContainerPort>,
    /// Name of the deployment to create
    pub name: String,
}

async fn create_deployment_impl(
    client: Client,
    params: DeploymentSpecParams,
) -> Result<Deployment, IntegrationOSError> {
    // let mut labels: BTreeMap<String, String> = BTreeMap::new();
    // labels.insert("app".to_owned(), name.to_owned());
    // labels.insert("database-type".to_owned(), "postgres".to_owned());

    // Definition of the deployment. Alternatively, a YAML representation could be used as well.
    let deployment: Deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(params.name.clone()),
            namespace: Some(params.namespace.clone()),
            labels: Some(params.labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(params.replicas),
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(params.labels.clone()),
            },
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: params.name,
                        image: Some(params.image),
                        // vec![ContainerPort {
                        //  container_port: 5005,
                        //  ..ContainerPort::default()
                        // }]
                        ports: Some(params.ports),
                        // vec![EnvVar {
                        //     name: "PORT".to_owned(),
                        //     value: Some("8080".to_owned()),
                        //     ..EnvVar::default()
                        // }]
                        env: Some(params.env),
                        ..Container::default()
                    }],
                    ..PodSpec::default()
                }),
                metadata: Some(ObjectMeta {
                    labels: Some(params.labels),
                    ..ObjectMeta::default()
                }),
            },
            ..DeploymentSpec::default()
        }),
        ..Deployment::default()
    };

    let deployment_api: Api<Deployment> = Api::namespaced(client, &params.namespace);
    deployment_api
        .create(&PostParams::default(), &deployment)
        .await
        .map_err(|e| InternalError::io_err(&format!("Could not create deployment: {e}"), None))
}

async fn delete_resource_impl<T>(
    client: Client,
    name: &str,
    namespace: &str,
) -> Result<Unit, IntegrationOSError>
where
    T: Resource<Scope = NamespaceResourceScope> + Clone + Debug + DeserializeOwned,
    <T as kube::Resource>::DynamicType: Default,
{
    let api: Api<T> = Api::namespaced(client, namespace);
    api.delete(name, &DeleteParams::default())
        .await
        .map_err(|e| InternalError::io_err(&format!("Could not delete deployment: {e}"), None))?;
    Ok(())
}

pub async fn delete_all_impl(
    client: Client,
    namespace: &str,
    name: &str,
) -> Result<Unit, IntegrationOSError> {
    delete_resource_impl::<Service>(client.clone(), name, namespace).await?;
    delete_resource_impl::<Deployment>(client.clone(), name, namespace).await?;

    Ok(())
}

pub async fn coordinator_impl(
    client: Client,
    service: ServiceSpecParams,
    deployment: DeploymentSpecParams,
) -> Result<Unit, IntegrationOSError> {
    if service.name != deployment.name || service.namespace != deployment.namespace {
        return Err(InternalError::invalid_argument(
            "Service and Deployment names and namespaces must match",
            None,
        ));
    }

    let namespace = service.namespace.clone();
    let name = service.name.clone();

    match create_service_impl(client.clone(), service).await {
        Ok(_service) => {
            tracing::info!("Created service {name} in namespace {namespace}");
            match create_deployment_impl(client.clone(), deployment).await {
                Ok(_deployment) => {
                    tracing::info!("Created deployment {name} in namespace {namespace}");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Error creating deployment. Cleaning up service {name} in namespace {namespace}: {e}");
                    delete_resource_impl::<Service>(client.clone(), &name, &namespace).await?;
                    Err(e)
                }
            }
        }
        Err(e) => {
            tracing::error!("Error creating service: {e}");
            Err(e)
        }
    }
}

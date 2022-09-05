use kube::{api::ListParams, client::Client, runtime::controller::Action, runtime::Controller, Api,CustomResourceExt};
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::api::core::v1::PodStatus;
use k8s_openapi::api::core::v1::PodCondition;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use k8s_openapi::Metadata;
use kube::api::{Patch, PatchParams,DeleteParams, ObjectMeta, PostParams};
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec, PodTemplateSpec};
use serde_json::{json, Value, Serializer};
use std::sync::{Arc,Mutex};
use futures::stream::StreamExt;
use kube::Resource;
use kube::ResourceExt;
use tokio::time::Duration;
use std::collections::BTreeMap;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use std::ops::{Deref, DerefMut};
use json_patch;


#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "kubeverage.tanisha.com",
    version = "v1",
    status = "BeverageStatus",
    kind = "Beverage",
    plural = "beverages",
    derive = "PartialEq",
    printcolumn = r#"{"name": "status", "jsonPath": ".status.order_status", "type": "string"}"#,
    printcolumn = r#"{"name":"Spec", "type":"string", "jsonPath":".spec.beveragename"}"#,
    printcolumn = r#"{"name":"Spec", "type":"string", "jsonPath":".spec.beveragetype"}"#,
    namespaced
)]
pub struct BeverageSpec {
    pub beveragename: String,
    pub beveragetype: String,
}
#[derive(Serialize, Deserialize, JsonSchema, Debug, PartialEq, Clone)]
pub struct BeverageStatus {
    order_status: String,
}


struct ContextData {
    client: Client,
}

impl ContextData {
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }

}
enum BeverageAction {
    Create,
    Delete,
    NoOp,
}

fn on_error(error: &Error, _context: Arc<ContextData>) -> Action {
    eprintln!("CRD Reconciliation error:\n{:?}", error);
    Action::requeue(Duration::from_secs(5))
}

fn determine_action(echo: &Beverage) -> BeverageAction {
    return if echo.meta().deletion_timestamp.is_some() {
        BeverageAction::Delete
    } else if echo
        .meta()
        .finalizers
        .as_ref()
        .map_or(true, |finalizers| finalizers.is_empty())
    {
        BeverageAction::Create
    } else {
        BeverageAction::NoOp
    };

}

pub async fn add(client: Client, name: &str, namespace: &str) -> Result<Beverage, Error> {
    let api: Api<Beverage> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": ["beverages.kubeverage.tanisha.com/finalizer"]
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    Ok(api.patch(name, &PatchParams::default(), &patch).await?)
}
pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<Beverage, Error> {
    let api: Api<Beverage> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": null
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    Ok(api.patch(name, &PatchParams::default(), &patch).await?)
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    
    #[error("Serde Json Error")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },
    
    #[error("Kubernetes reported error: {source}")]
    KubeError {
        #[from]
        source: kube::Error,
    },

    #[error("Invalid Echo CRD: {0}")]
    UserInputError(String),
}
pub async fn deploy(
    client: Client,
    beveragename: &str,
) -> Result<Pod, kube::Error> {
    
    let containername: &str = "nginx";
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), beveragename.to_owned());
    
    
    let namespace: &str = "default";
    let port_type: &str = "http";
    
    
    let pod: Pod = Pod {
            metadata: ObjectMeta {
                name: Some(beveragename.to_owned()),
                namespace: Some(namespace.to_owned()),
                labels: Some(labels.clone()),
                ..ObjectMeta::default()
            },
            spec: Some(PodSpec{
                containers: vec![Container{
                    name: containername.to_owned(),
                    image: Some("nginx:latest".to_owned()),
                    ports: Some(vec![ContainerPort {
                        container_port: 80,
                        name: Some(port_type.to_owned()),
                        ..ContainerPort::default() 
                    }]),
                    ..Container::default()
                }],
                ..PodSpec::default()
            }),
            ..Pod::default()               
    };
    // Create the pod defined above
    let pod_api: Api<Pod> = Api::default_namespaced(client);
    pod_api.create(&PostParams::default(), &pod).await
}

pub async fn delete_pod(client: Client, name: &str) -> Result<(), Error> {
    let namespace: &str = "default";
    let api: Api<Pod> = Api::namespaced(client, namespace);
    api.delete(name, &DeleteParams::default()).await?;
    Ok(())
}

async fn reconcile(beverage: Arc<Beverage>, context: Arc<ContextData>) -> Result<Action, Error> {
    let client: Client = context.client.clone(); 
    let namespace: String = match beverage.namespace() {
        None => {
            
            return Err(Error::UserInputError(
                "Expected Beverage resource to be namespaced. Can't deploy pod to an unknown namespace."
                    .to_owned(),
            ));
        }
       
        Some(namespace) => namespace,
    };

    return match determine_action(&beverage) {
        BeverageAction::Create => {

            let name: &str = &beverage.name_any(); 
            println!("{}", name);

            let beverage_api: Api<Beverage> = Api::default_namespaced(client.clone());
            let patch_params = PatchParams::default();

            let status = json!({ "status": BeverageStatus { order_status: "Your order is being prepared".to_owned() } });


            println!("Hi: 206");
            let updated = beverage_api.patch_status(&name,&patch_params,&Patch::Merge(&status)).await.map_err(|source| Error::KubeError {
                source,})?;

            println!("Hi: 211");
            println!("Patched status {:?} for {}", updated.status, updated.name_any());
            
            add(client.clone(), name, &namespace).await?;
            deploy(client, name).await?;
            
            
            let patch_params = PatchParams::default();

            let status = json!({ "status": BeverageStatus { order_status: "Your order is ready".to_owned() } });

            println!("Hi: 206");
            let updated = beverage_api.patch_status(&name,&patch_params,&Patch::Merge(&status)).await.map_err(|source| Error::KubeError {
                source,})?;
            println!("Hi: 211");
            println!("Patched status {:?} for {}", updated.status, updated.name_any());
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        BeverageAction::Delete => {
            delete(client.clone(), &beverage.spec.beveragename, &namespace).await?;

            delete_pod(client, &beverage.spec.beveragename).await?;
            Ok(Action::await_change()) 
        }
        BeverageAction::NoOp => Ok(Action::requeue(Duration::from_secs(10))),
    };
}

async fn create_crd (client: &Client){
    let new_crd_api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let beveragecrd = Beverage::crd();
    println!(
        "Creating CRD: {}",
        serde_json::to_string_pretty(&beveragecrd).unwrap()
    );

    let pp = PostParams::default();
    match new_crd_api.create(&pp, &beveragecrd).await {
        Ok(o) => {
            println!("CRD Created !");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        Err(e) => {
            println!("Failed to create CRD, error {}", e);
            tokio::time::sleep(Duration::from_secs(1)).await;
            std::process::exit(1);
        }
    }
}
#[tokio::main]
async fn main() {
    let client: Client = Client::try_default()
                        .await
                        .expect("Expected a valid KUBECONFIG env variable");
    let new_crd_api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let beveragecrd = Beverage::crd();
    println!(
        "Creating CRD: {}",
        serde_json::to_string_pretty(&beveragecrd).unwrap()
    );

    let pp = PostParams::default();
    match new_crd_api.create(&pp, &beveragecrd).await {
        Ok(o) => {
            println!("CRD Created !");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        Err(e) => {
            println!("Failed to create CRD, error {}", e);
            tokio::time::sleep(Duration::from_secs(1)).await;
            std::process::exit(1);
        }
    }
    
    let crd_api: Api<Beverage> = Api::all(client.clone());
    let context: Arc<ContextData> = Arc::new(ContextData::new(client.clone()));
    Controller::new(crd_api.clone(), ListParams::default())
        .run(reconcile, on_error, context)
        .for_each(|reconciliation_result| async move {
            match reconciliation_result {
                Ok(beverage_resource) => {
                    println!("Reconciliation successful. Resource: {:?}", beverage_resource);
                    println!("Reconciliation successful");
                }
                Err(reconciliation_err) => {
                    eprintln!("Reconciliation error: {:?}", reconciliation_err);
                    println!("Reconciliation error");
                }
            }
        })
        .await;
    println!("Hello, world!");
}
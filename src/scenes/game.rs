use std::rc::Rc;
use std::cell::RefCell;
use rand::prelude::*;
use crate::models::config::Config;
use crate::assets::Assets;
use tetra::{Context, graphics, audio, window};
use specs::prelude::*;
use tetra::math::Vec2;
use crate::components::{Position, Renderable, Rotation, Hidden, Scaleable};
use crate::systems::*;
use tetra::graphics::DrawParams;
use crate::ressources::{camera::CameraRessource, Gamestate, State};
use crate::scenes::manager::{Transition, Scene};
use crate::{arena, components, ressources};
use crate::auxiliary::*;
use std::time::{Instant, Duration};
use tetra::audio::{SoundInstance, Sound};

#[allow(dead_code)]
pub struct GameScene {
	world: World,
	alive: Instant,
	alive_sum: Duration,
	measurement_running: bool,
	dispatcher: Dispatcher<'static, 'static>,
	config: Rc<Config>,
	assets: Rc<RefCell<Assets>>,
	background_music_instance: SoundInstance,
}

impl GameScene {
	pub fn new(ctx: &mut Context, config: Rc<Config>, assets: Rc<RefCell<Assets>>) -> tetra::Result<GameScene> {
		let mut world= World::new();
		components::register(&mut world);
		arena::create(&mut world);
		ressources::insert(ctx, &mut world);
		audio::set_master_volume(ctx, config.master_volume);
		let background_music = Sound::from_file_data(include_bytes!("../../assets/music/Star Flow.mp3"));
		let background_music_instance = background_music.spawn(ctx)?;
		background_music_instance.set_repeating(true);
		background_music_instance.play();
		background_music_instance.set_volume(0.2);

		Ok(GameScene {
			world,
			alive: Instant::now(),
			alive_sum: Duration::new(0,0),
			measurement_running: false,
			dispatcher: create_dispatcher(),
			assets,
			config,
			background_music_instance,
		})
	}
}

impl Scene for GameScene {
	fn update(&mut self, ctx: &mut Context) -> tetra::Result<Transition> {
		// automatic
		self.dispatcher.dispatch(&self.world);

		// manual
		lifetime_system::cull_deads(&mut self.world);
		input_system::update(&mut self.world, ctx);

		if self.world.read_resource::<Gamestate>().state == State::Init{
			self.world.write_resource::<Gamestate>().state = State::Running;
			{
				arena::reset(&mut self.world, SeedableRng::from_seed(SEED));
				self.alive = Instant::now();
				self.alive_sum = Duration::new(0,0);
				self.measurement_running = true;
			}
		}
		if self.world.write_resource::<Gamestate>().state != State::Running && self.measurement_running{
			self.alive_sum += self.alive.elapsed();
			self.measurement_running = false;
			let min = (self.alive_sum / 60).as_secs();
			let sec = self.alive_sum - Duration::from_secs((min*60) as u64);
			self.assets.borrow_mut().get_text_mut().set_content(format!("{:?} min {:.2} sec", min, sec.as_secs_f32()));
		}
		if self.world.read_resource::<Gamestate>().state == State::Quit{
			window::quit(ctx);
		}

		Ok(Transition::None)
	}

	fn draw(&mut self, ctx: &mut Context) -> tetra::Result<Transition> {
		let camera = self.world.read_resource::<CameraRessource>();
		graphics::clear(ctx, self.config.clear_color);

		//ecs rendering
		let positions = self.world.read_storage::<Position>();
		let renderables = self.world.read_storage::<Renderable>();
		let entities = self.world.entities();
		let rotations = self.world.read_storage::<Rotation>();
		let scaleable = self.world.read_storage::<Scaleable>();
		let hidden = self.world.read_storage::<Hidden>();

		let mut data = (&positions, &renderables, &entities, !&hidden).par_join().collect::<Vec<_>>();
		data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order) );

		for (pos, render, entity, _) in data.iter().rev() {
			let rot = match rotations.get(*entity){
				Some(r) => r.value as f32,
				_ => 0.0
			};
			let scale = match scaleable.get(*entity){
				Some(s) => s.value,
				_ => Vec2::one(),
			};
			let draw_params = DrawParams::new()
				.position(pos.value - camera.offset)
				.color(to_tetra_color(render.color))
				.rotation(degrees_to_radians(rot))
				.scale(scale)
				.origin(render.origin);
			self.assets.borrow().draw(ctx, render.texture_id, draw_params);
			if self.world.read_resource::<Gamestate>().state == State::Dead{
				self.assets.borrow().draw(ctx, 800, DrawParams::new()
					.position(camera.window_half)
					.origin(Vec2::new(128.0,32.0))
				);
				self.assets.borrow().draw_text(ctx, 0, DrawParams::new()
					.position(camera.window_half-Vec2::new(120.0,-40.0))
				);
			}
			if self.world.read_resource::<Gamestate>().state == State::Start{
				self.assets.borrow().draw(ctx, 801, DrawParams::new()
					.position(camera.window_half)
					.origin(Vec2::new(128.0,32.0))
				);
			}
			if self.world.read_resource::<Gamestate>().state == State::Pause{
				self.assets.borrow().draw(ctx, 802, DrawParams::new()
					.position(camera.window_half)
					.origin(Vec2::new(128.0,32.0))
				);
			}
		}

		Ok(Transition::None)
	}
}
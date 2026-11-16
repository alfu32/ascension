-- unit.missile_cruiser@information: Missile cruiser (mesh.missile_cruiser_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.missile_cruiser_information','Missile cruiser');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.missile_cruiser_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.haze_grey',0,'','{"yaw":-90,"y":-0.18,"w":4.6,"d":1,"h":0.6,"sheer":0.12,"segments":12,"float":1}'),
  ('deck','box','mat.tarmac',0,'hull','{"y":0.54,"w":3.4,"d":0.92,"h":0.04}'),
  ('deck_ends','box','mat.tarmac',0,'hull','{"x":-1.85,"y":0.54,"w":0.5,"d":0.62,"h":0.04,"repeat":2,"dx":3.7,"far":0}'),
  ('boot','box','mat.cloth',1,'hull','{"y":0.4,"z":-0.49,"w":3,"d":0.03,"h":0.1,"repeat":2,"dz":0.98,"far":0}'),
  ('superstructure','box','mat.haze_grey',0,'hull','{"x":0.25,"y":0.56,"w":1.5,"d":0.8,"h":0.48}'),
  ('bridge','box','mat.haze_grey',0,'hull','{"x":0.5,"y":1.04,"w":0.7,"d":0.62,"h":0.32,"taper":0.1,"windows":0,"side_windows":0}'),
  ('bridge_band','box','mat.cloth',1,'hull','{"x":0.5,"y":1.3,"w":0.62,"d":0.56,"h":0.07}'),
  ('bridge_windows','box','mat.composite',0,'hull','{"x":0.86,"y":1.18,"w":0.02,"d":0.5,"h":0.08,"far":0}'),
  ('panel_fore','box','mat.haze_grey',0,'hull','{"x":0.88,"y":1.08,"w":0.03,"d":0.36,"h":0.34,"roll":-12,"tint":"#c8ccd0","far":0}'),
  ('panel_s','box','mat.haze_grey',0,'hull','{"x":0.5,"y":1.08,"z":0.32,"w":0.36,"d":0.03,"h":0.34,"pitch":12,"tint":"#c8ccd0","far":0}'),
  ('panel_n','box','mat.haze_grey',0,'hull','{"x":0.5,"y":1.08,"z":-0.32,"w":0.36,"d":0.03,"h":0.34,"pitch":-12,"tint":"#c8ccd0","far":0}'),
  ('mast','cylinder','mat.haze_grey',0,'hull','{"x":0.25,"y":1.36,"w":0.1,"h":0.8,"segments":5}'),
  ('radar_dome','dome','mat.white_metal',0,'mast','{"y":0.8,"w":0.26,"h":0.2,"segments":7}'),
  ('radar','propeller','mat.composite',0,'mast','{"y":0.55,"w":0.62,"blades":2,"chord":0.1,"pitch":-90}'),
  ('mast_top','cylinder','mat.cloth',1,'mast','{"y":0.4,"w":0.12,"h":0.12,"segments":5,"far":0}'),
  ('stack','box','mat.cloth',1,'hull','{"x":-0.4,"y":1.04,"w":0.36,"d":0.4,"h":0.32,"taper":0.12}'),
  ('stack_cap','box','mat.composite',0,'stack','{"y":0.32,"w":0.3,"d":0.34,"h":0.06,"far":0}'),
  ('cells_fore','box','mat.cast_iron',0,'hull','{"x":1.18,"y":0.56,"w":0.56,"d":0.56,"h":0.04}'),
  ('hatches_fore','box','mat.composite',0,'hull','{"x":0.98,"y":0.6,"z":-0.17,"w":0.1,"d":0.1,"h":0.015,"repeat":4,"dx":0.13,"far":0}'),
  ('hatches_fore_2','box','mat.composite',0,'hull','{"x":0.98,"y":0.6,"z":0.17,"w":0.1,"d":0.1,"h":0.015,"repeat":4,"dx":0.13,"far":0}'),
  ('cells_aft','box','mat.cast_iron',0,'hull','{"x":-0.9,"y":0.56,"w":0.52,"d":0.56,"h":0.04,"far":0}'),
  ('hatches_aft','box','mat.composite',0,'hull','{"x":-1.08,"y":0.6,"z":-0.17,"w":0.1,"d":0.1,"h":0.015,"repeat":4,"dx":0.12,"far":0}'),
  ('gun_house','box','mat.haze_grey',0,'hull','{"yaw":90,"x":1.72,"y":0.58,"w":0.3,"d":0.36,"h":0.16,"taper":0.45,"traverse":30}'),
  ('gun','cylinder','mat.haze_grey',0,'gun_house','{"y":0.07,"z":0.12,"w":0.05,"h":0.5,"segments":5,"pitch":90,"recoil":0.06}'),
  ('helipad','cylinder','mat.cloth',1,'hull','{"x":-1.72,"y":0.58,"w":0.5,"h":0.01,"segments":10,"far":0}'),
  ('ensign','box','mat.cloth',1,'hull','{"x":-2.25,"y":0.98,"w":0.34,"d":0.015,"h":0.22,"far":0}'),
  ('ensign_staff','cylinder','mat.haze_grey',0,'hull','{"x":-2.12,"y":0.58,"w":0.03,"h":0.6,"segments":4,"far":0}'),
  ('spark','cone','mat.flash',0,'cells_fore','{"y":0.04,"w":0.144,"h":0.144,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.384,"w":0.432,"h":0.384,"segments":6,"pitch":180,"far":0}'),
  ('missile','cylinder','mat.white_metal',0,'spark','{"y":1.2,"w":0.084,"h":0.36,"segments":5,"far":0}'),
  ('trail','cone','mat.canvas',0,'missile','{"y":-1.08,"w":0.24,"h":1.1,"segments":5,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.missile_cruiser_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.missile_cruiser'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.missile_cruiser_information'), (SELECT id FROM def WHERE key='mat.haze_grey'), 100);
